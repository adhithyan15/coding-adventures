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
//! | `"print"` | Prints all args to stdout; returns `Null` |
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
use crate::errors::VMError;
use crate::value::Value;

/// Signature for a built-in handler: takes a slice of arguments, returns a
/// result value or a VMError.
pub type BuiltinFn = Box<dyn Fn(&[Value]) -> Result<Value, VMError> + Send + Sync>;

/// Registry of named built-in function handlers.
pub struct BuiltinRegistry {
    handlers: HashMap<String, BuiltinFn>,
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
}
