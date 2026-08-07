//! [`BuiltinRegistry`] — pre-registered built-in function handlers.
//!
//! Language frontends can register named built-in functions that are called
//! via the `call_builtin` opcode.  The VM looks up the handler by name in
//! O(1) and delegates the call.
//!
//! # Built-ins pre-registered by default
//!
//! | Name | Behaviour |
//! |------|-----------|
//! | `"noop"` | No-op; returns `Null` |
//! | `"assert_eq"` | Panics if `args[0] != args[1]`; returns `Null` |
//! | `"print"` | Prints all args to stdout with a trailing newline; returns `Null` |
//! | `"print_str"` | Prints one string to stdout with no implicit newline; returns `Null` |
//!
//! # Example
//!
//! ```
//! use vm_core::builtins::BuiltinRegistry;
//! use vm_core::value::Value;
//!
//! let mut reg = BuiltinRegistry::new();
//! reg.register("double", |args| {
//!     let n = args.first().and_then(|v| v.as_i64()).unwrap_or(0);
//!     Ok(Value::Int(n * 2))
//! });
//! let result = reg.call("double", &[Value::Int(21)]).unwrap();
//! assert_eq!(result, Value::Int(42));
//! ```

use std::collections::HashMap;
use std::io::{self, Write};
use crate::errors::VMError;
use crate::value::Value;

/// Signature for a built-in handler: takes a slice of arguments, returns a
/// result value or a VMError.
pub type BuiltinFn = Box<dyn Fn(&[Value]) -> Result<Value, VMError> + Send + Sync>;

/// Registry of named built-in function handlers.
pub struct BuiltinRegistry {
    handlers: HashMap<String, BuiltinFn>,
}

/// The dynamic arithmetic primitive shared by the `+`/`-`/`*` builtins (E6d-2) —
/// see their registration in [`BuiltinRegistry::new`]. Two same-kind numeric
/// operands compute a result of that kind: integers wrap on overflow (the i64
/// tagged-value model the code-gen backends use), floats compute in `f64`. Any
/// other operand pair is a clean type error, never a panic.
fn dyn_arith(sym: &str, args: &[Value]) -> Result<Value, VMError> {
    if args.len() != 2 {
        return Err(VMError::Custom(format!(
            "`{sym}` requires exactly 2 arguments, got {}",
            args.len()
        )));
    }
    match (&args[0], &args[1]) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(match sym {
            "+" => a.wrapping_add(*b),
            "-" => a.wrapping_sub(*b),
            "*" => a.wrapping_mul(*b),
            _ => unreachable!("dyn_arith is only registered for + - *"),
        })),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(match sym {
            "+" => a + b,
            "-" => a - b,
            "*" => a * b,
            _ => unreachable!("dyn_arith is only registered for + - *"),
        })),
        (a, b) => Err(VMError::TypeError {
            expected: "two numbers of the same kind".into(),
            actual: format!("{} and {}", a.iir_type_name(), b.iir_type_name()),
            context: format!("dynamic `{sym}`"),
        }),
    }
}

impl BuiltinRegistry {
    /// Create a new registry pre-loaded with `noop`, `assert_eq`, and `print`.
    pub fn new() -> Self {
        let mut reg = BuiltinRegistry {
            handlers: HashMap::new(),
        };
        // noop — used as a placeholder / timing baseline.
        reg.register("noop", |_args| Ok(Value::Null));

        // assert_eq — raises VMError::Custom if args[0] != args[1].
        // Useful for writing self-checking IIR programs in tests.
        reg.register("assert_eq", |args| {
            if args.len() < 2 {
                return Err(VMError::Custom(
                    "assert_eq requires 2 arguments".into(),
                ));
            }
            if args[0] != args[1] {
                Err(VMError::Custom(format!(
                    "assert_eq failed: {:?} != {:?}",
                    args[0], args[1]
                )))
            } else {
                Ok(Value::Null)
            }
        });

        // print — writes all args to stdout separated by spaces.
        reg.register("print", |args| {
            let parts: Vec<String> = args.iter().map(|v| v.to_string()).collect();
            println!("{}", parts.join(" "));
            Ok(Value::Null)
        });

        // print_str — writes a single string to stdout with no implicit newline.
        reg.register("print_str", |args| {
            let s = args.first()
                .and_then(Value::as_str)
                .ok_or_else(|| VMError::TypeError {
                    expected: "str".into(),
                    actual: args.first().map(Value::iir_type_name).unwrap_or("missing").into(),
                    context: "print_str".into(),
                })?;
            print!("{s}");
            io::stdout()
                .flush()
                .map_err(|e| VMError::Custom(format!("print_str flush failed: {e}")))?;
            Ok(Value::Null)
        });

        // `=` — the dynamic-equality primitive (E6d). The tagged/structural
        // code-gen backends lower it to `__dyn_eq` / a typed compare that unboxes
        // its operands first; on the generic VM a `Value` is already the dynamic
        // value, so equality is a direct `Value` compare (`Int == Int`,
        // `Str == Str`, …), returning a boolean. Twig union `match` uses it to test
        // a variant's integer tag against each arm's tag; the boolean result feeds
        // a `jmp_if_false`. (Distinct from the `cmp_eq` *opcode*, which the typed
        // frontends emit for statically-typed comparisons.)
        reg.register("=", |args| {
            if args.len() != 2 {
                return Err(VMError::Custom(format!(
                    "`=` requires exactly 2 arguments, got {}",
                    args.len()
                )));
            }
            Ok(Value::Bool(args[0] == args[1]))
        });

        // Dynamic arithmetic primitives (E6d-2) — the same `any`-typed `+`/`-`/`*`
        // the frontend emits as a `call_builtin` when an operand's static type is
        // `any` (e.g. a value read from a cons cell or a bound `match` field). The
        // tagged/structural backends route these to `__dyn_add`/… (unbox, compute,
        // rebox); on the generic VM a `Value` is already the value, so it is a
        // direct compute. Integer operands wrap on overflow (the i64 tagged model);
        // floats compute in `f64`. A non-numeric operand is a clean type error.
        for sym in ["+", "-", "*"] {
            reg.register(sym, move |args| dyn_arith(sym, args));
        }

        reg
    }

    /// Register a named built-in handler.
    ///
    /// If a handler with the same name already exists it is replaced.
    pub fn register<F>(&mut self, name: impl Into<String>, handler: F)
    where
        F: Fn(&[Value]) -> Result<Value, VMError> + Send + Sync + 'static,
    {
        self.handlers.insert(name.into(), Box::new(handler));
    }

    /// Call the named built-in with the given arguments.
    ///
    /// Returns `VMError::UnknownOpcode` if no handler is registered for
    /// `name` (mirrors the Python vm-core behaviour for unregistered builtins).
    pub fn call(&self, name: &str, args: &[Value]) -> Result<Value, VMError> {
        match self.handlers.get(name) {
            Some(handler) => handler(args),
            None => Err(VMError::UnknownOpcode(format!("builtin {name:?}"))),
        }
    }

    /// Return `true` if a handler is registered for `name`.
    pub fn has(&self, name: &str) -> bool {
        self.handlers.contains_key(name)
    }
}

impl Default for BuiltinRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for BuiltinRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuiltinRegistry")
            .field("handlers", &self.handlers.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_returns_null() {
        let reg = BuiltinRegistry::new();
        assert_eq!(reg.call("noop", &[]).unwrap(), Value::Null);
    }

    #[test]
    fn assert_eq_passes() {
        let reg = BuiltinRegistry::new();
        assert_eq!(
            reg.call("assert_eq", &[Value::Int(42), Value::Int(42)]).unwrap(),
            Value::Null
        );
    }

    #[test]
    fn assert_eq_fails() {
        let reg = BuiltinRegistry::new();
        assert!(reg.call("assert_eq", &[Value::Int(1), Value::Int(2)]).is_err());
    }

    // ── E6d dynamic-dispatch builtins ───────────────────────────────────────

    #[test]
    fn dyn_eq_returns_boolean() {
        let reg = BuiltinRegistry::new();
        assert_eq!(reg.call("=", &[Value::Int(1), Value::Int(1)]).unwrap(), Value::Bool(true));
        assert_eq!(reg.call("=", &[Value::Int(1), Value::Int(2)]).unwrap(), Value::Bool(false));
        // Cross-kind values are simply unequal, never an error (dynamic equality).
        assert_eq!(reg.call("=", &[Value::Int(0), Value::Null]).unwrap(), Value::Bool(false));
    }

    #[test]
    fn dyn_arith_computes_and_wraps() {
        let reg = BuiltinRegistry::new();
        assert_eq!(reg.call("+", &[Value::Int(20), Value::Int(22)]).unwrap(), Value::Int(42));
        assert_eq!(reg.call("-", &[Value::Int(50), Value::Int(8)]).unwrap(), Value::Int(42));
        assert_eq!(reg.call("*", &[Value::Int(6), Value::Int(7)]).unwrap(), Value::Int(42));
        // Integer overflow wraps (the i64 tagged-value model), never panics.
        assert_eq!(reg.call("+", &[Value::Int(i64::MAX), Value::Int(1)]).unwrap(), Value::Int(i64::MIN));
        // Floats compute in f64.
        assert_eq!(reg.call("+", &[Value::Float(1.5), Value::Float(2.0)]).unwrap(), Value::Float(3.5));
    }

    #[test]
    fn dyn_arith_rejects_bad_arity_and_types() {
        let reg = BuiltinRegistry::new();
        assert!(reg.call("+", &[Value::Int(1)]).is_err(), "wrong arity");
        assert!(reg.call("=", &[Value::Int(1)]).is_err(), "wrong arity");
        assert!(
            matches!(reg.call("*", &[Value::Int(1), Value::Null]), Err(VMError::TypeError { .. })),
            "non-numeric operand is a clean type error"
        );
    }

    #[test]
    fn unknown_builtin_errors() {
        let reg = BuiltinRegistry::new();
        assert!(matches!(reg.call("no_such_fn", &[]), Err(VMError::UnknownOpcode(_))));
    }

    #[test]
    fn custom_builtin_registered_and_called() {
        let mut reg = BuiltinRegistry::new();
        reg.register("double", |args| {
            let n = args.first().and_then(|v| v.as_i64()).unwrap_or(0);
            Ok(Value::Int(n * 2))
        });
        assert_eq!(reg.call("double", &[Value::Int(21)]).unwrap(), Value::Int(42));
    }

    #[test]
    fn print_str_requires_a_string() {
        let reg = BuiltinRegistry::new();
        let err = reg.call("print_str", &[Value::Int(1)]).unwrap_err();
        assert!(matches!(err, VMError::TypeError { .. }));
    }
}
