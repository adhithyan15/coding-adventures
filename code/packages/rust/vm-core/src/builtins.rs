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
//! | `"print_i64"` | Prints one integer in decimal **plus `\n`** to stdout (LANG83 standard builtin) |
//! | `"putchar"` | Writes the low byte of one integer to stdout (LANG83 standard builtin) |
//! | `"write_i64"` | Prints one integer in decimal, **no** newline (LANG83 PR1b standard builtin) |
//!
//! ## The dynamic builtins (what a lisp-family frontend emits)
//!
//! A dynamically-typed frontend (Twig is the one that matters today) cannot
//! always prove its operands are integers. `(define (f x) (if (< x 0) 42 7))`
//! has no type for `x`, so the compiler cannot emit the typed `cmp_lt` opcode;
//! it emits `call_builtin "<", x, 0` and leaves the decision to run time. The
//! code-gen backends rewrite those calls in `iir-builtin-lowering`; the generic
//! VM runs them directly, because a [`Value`] already *is* the dynamic value.
//! So every source-level builtin the frontend can leave in the IIR must be
//! registered here, or the VM stops with `UnknownOpcode("builtin \"<\"")`.
//!
//! | Name | Accepts | Result |
//! |------|---------|--------|
//! | `"+"` `"-"` `"*"` | two `Int`s (wrapping) or two `Float`s | same kind |
//! | `"<"` `"<="` `">"` `">="` | two `Int`s or two `Float`s | `Bool` |
//! | `"="` `"equal?"` | any two values | `Bool` (`Value` equality) |
//! | `"pair?"` | any one value | `Bool`: is it a non-null heap reference? |
//!
//! An operand outside the "Accepts" column is a [`VMError::TypeError`], never
//! a panic and never a silent coercion: `(< "a" 1)` is an error, as it is on
//! `twig-vm`, whose `dynval-runtime` rejects a non-integer operand of `<`
//! ("`<` expects integers").
//!
//! **Known gap (#18470):** on this VM nil and interned symbols are themselves
//! `Value::Int`s (nil is `Int(0)`, a symbol is `Int(SYMBOL_ID_BASE + id)`), so
//! they reach the integer arm: `(< '() 1)` is `#t` here where `twig-vm` raises.
//! Closing that needs a representation for nil and symbols that is not an
//! integer, the same decision `number?` / `symbol?` are waiting on.
//!
//! There is no float writer. LANG83 PR6a's `write_f64_py` / `write_f64_js`
//! (a float in Python's / JavaScript's layout) were removed in PR7e, because
//! a VM builtin may not carry one language's rules: a compiler prints a float
//! with its own module-local helper over the `f64_class` / `f64_digits` /
//! `f64_exp10` ops (see `crate::float_format`). No default builtin's name
//! names a language (`interpreter_ir::op_registry::language_name_violation`,
//! checked by `default_builtins_name_no_language`).
//!
//! ## One ordered output stream (LANG83)
//!
//! `print_str`, `print_i64`, `write_i64` and `putchar` all write through the **same**
//! `std::io::stdout()` handle, in call order, so a program that mixes them sees
//! one ordered byte stream — never "all the integers, then all the bytes". An
//! embedder that wants to capture output re-registers all four names with
//! closures that append to *one* shared buffer (the `lang-aot` matrix harness
//! does exactly that); registering separate buffers per builtin and choosing
//! between them afterwards is the bug LANG83 removed.
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

/// The dynamic ordering primitive shared by `<` / `<=` / `>` / `>=` — the
/// comparison sibling of [`dyn_arith`], with the same operand contract.
///
/// | operands            | result                                  |
/// |---------------------|-----------------------------------------|
/// | `Int a`, `Int b`    | `Bool(a OP b)`, a **signed** i64 compare |
/// | `Float a`, `Float b`| `Bool(a OP b)`, IEEE (`NaN` → `false`)   |
/// | anything else       | `VMError::TypeError`                     |
///
/// Three choices here are deliberate, and each one is "do what the other
/// engines already do", not a new rule:
///
/// * **Signed.** `(< -5 0)` is true. `twig-vm` compares two `i64`s, and the
///   code-gen backends lower the same call to `cmp_lt` on `i64` (signed on
///   the native ones too once #18453's fix lands).
/// * **No `Bool` as a number.** [`Value::as_i64`] reads `true` as `1`, which is
///   right for a typed `bool` register but wrong here: Scheme's `(< #t 2)` is a
///   type error, and `twig-vm`'s `as_int` rejects a boolean. So this matches on
///   `Value::Int` itself rather than calling `as_i64`.
/// * **Two numbers of one kind.** Twig has only integers, so for every Twig
///   program this is `twig-vm`'s integer-only rule. The `Float` arm keeps the
///   comparisons in step with `+`/`-`/`*` above, which already accept two
///   floats; a mixed `Int`/`Float` pair is refused by both, for the same
///   reason (no implicit numeric promotion in the dynamic contract).
///
/// Only the binary form exists. Scheme's chained `(< a b c)` would need its own
/// rule (`a<b ∧ b<c`), and neither `dynval-runtime` nor the code-gen lowering
/// implements it, so a third argument is an arity error everywhere.
fn dyn_compare(sym: &str, args: &[Value]) -> Result<Value, VMError> {
    if args.len() != 2 {
        return Err(VMError::Custom(format!(
            "`{sym}` requires exactly 2 arguments, got {}",
            args.len()
        )));
    }
    // One table, two numeric kinds: pick the operator once, apply it to
    // whichever pair of numbers we were handed. `PartialOrd` gives IEEE
    // semantics for `f64` (every comparison with a NaN is false).
    fn apply<T: PartialOrd>(sym: &str, a: T, b: T) -> bool {
        match sym {
            "<" => a < b,
            "<=" => a <= b,
            ">" => a > b,
            ">=" => a >= b,
            _ => unreachable!("dyn_compare is only registered for < <= > >="),
        }
    }
    match (&args[0], &args[1]) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(apply(sym, *a, *b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(apply(sym, *a, *b))),
        (a, b) => Err(VMError::TypeError {
            expected: "two numbers of the same kind".into(),
            actual: format!("{} and {}", a.iir_type_name(), b.iir_type_name()),
            context: format!("dynamic `{sym}`"),
        }),
    }
}

/// `pair?` on the generic VM: is this value a non-null heap reference?
///
/// It looks only at the value's kind; it does not ask the heap whether the
/// block is live (a dangling host-held `HeapRef` would also answer `#t`).
///
/// On this VM a cons cell is what `iir-builtin-lowering::lower_heap_builtins`
/// turns `cons` into: an `alloc` + two `field_store`s, so a cell is a
/// [`Value::HeapRef`]. Nil is the null reference (or the integer `0` a
/// `const 0 : ref<LispyPair>` produces — the same two shapes `is_null` accepts),
/// and every atom (`Int`, `Float`, `Bool`, `Str`) is not a cell at all.
///
/// | argument                     | `pair?` |
/// |------------------------------|---------|
/// | `(cons 1 2)` → non-null `HeapRef` | `#t` |
/// | `'()` → null `HeapRef` / `Int(0)`  | `#f` |
/// | `42`, `#t`, `"s"`, `1.5`         | `#f` |
///
/// This is the *structural* answer: anything the shared lowering builds out of
/// cells answers `#t`. That includes a record, which is what Twig relies on —
/// a record's own `<name>?` predicate is compiled to `(pair? v)`. It also
/// includes a closure once `lower_closures_to_heap` has turned it into a cell
/// chain, where `twig-vm` (which tags closures separately) says `#f`. A
/// `HeapRef` carries no "this is a cons" tag, so the VM cannot draw that line;
/// no Twig program in the matrix asks `pair?` of a closure. For the same
/// reason a record predicate such as `point?` is `#t` for any cell — a cons,
/// a closure, or another record type — and the accessor after it reads that
/// cell's fields (bounds-checked, so a wrong answer, not an unsafe read).
fn dyn_is_pair(args: &[Value]) -> Result<Value, VMError> {
    match args {
        [v] => Ok(Value::Bool(matches!(v, Value::HeapRef(r) if !r.is_null()))),
        _ => Err(VMError::Custom(format!(
            "`pair?` requires exactly 1 argument, got {}",
            args.len()
        ))),
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

        // print_i64 — LANG83 standard output builtin: decimal value + "\n".
        // Before LANG83 every embedder had to register this itself, so a bare
        // `VMCore` failed any BASIC/Oct/ALGOL program with UnknownOpcode.
        reg.register("print_i64", |args| {
            let n = args.first().and_then(Value::as_i64).ok_or_else(|| VMError::TypeError {
                expected: "integer".into(),
                actual: args.first().map(Value::iir_type_name).unwrap_or("missing").into(),
                context: "print_i64".into(),
            })?;
            let mut out = io::stdout();
            writeln!(out, "{n}")
                .and_then(|()| out.flush())
                .map_err(|e| VMError::Custom(format!("print_i64 write failed: {e}")))?;
            Ok(Value::Null)
        });

        // write_i64 — LANG83 PR1b standard output builtin: decimal value, NO
        // newline (Perl `print`, Python `print(x, end="")`). Same handle and
        // flush discipline as print_i64, so it shares the one ordered stream.
        reg.register("write_i64", |args| {
            let n = args.first().and_then(Value::as_i64).ok_or_else(|| VMError::TypeError {
                expected: "integer".into(),
                actual: args.first().map(Value::iir_type_name).unwrap_or("missing").into(),
                context: "write_i64".into(),
            })?;
            let mut out = io::stdout();
            write!(out, "{n}")
                .and_then(|()| out.flush())
                .map_err(|e| VMError::Custom(format!("write_i64 write failed: {e}")))?;
            Ok(Value::Null)
        });

        // putchar — LANG83 standard output builtin: the low byte of an integer.
        reg.register("putchar", |args| {
            let n = args.first().and_then(Value::as_i64).ok_or_else(|| VMError::TypeError {
                expected: "integer".into(),
                actual: args.first().map(Value::iir_type_name).unwrap_or("missing").into(),
                context: "putchar".into(),
            })?;
            let mut out = io::stdout();
            out.write_all(&[(n & 0xFF) as u8])
                .and_then(|()| out.flush())
                .map_err(|e| VMError::Custom(format!("putchar write failed: {e}")))?;
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

        // `equal?` — the lisp-family spelling of dynamic value equality. Symbol
        // literals are interned before the generic VM sees them, so same-name
        // symbols are the same `Value::Int` and distinct names are different ints.
        // Keeping this beside `=` also makes the VM reusable by any frontend that
        // emits the Scheme spelling instead of requiring a language-specific runner.
        reg.register("equal?", |args| {
            if args.len() != 2 {
                return Err(VMError::Custom(format!(
                    "`equal?` requires exactly 2 arguments, got {}",
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

        // Dynamic ordering (#18463) — `<`/`<=`/`>`/`>=` over `any`. Twig emits
        // these whenever an operand's type is not statically `i64`, which is
        // every comparison on an un-annotated function parameter:
        // `(define (f x) (if (< x 0) 42 7))` compiles to
        // `call_builtin "<", x, 0`. Before this the VM and JIT matrix columns
        // stopped there with `UnknownOpcode("builtin \"<\"")` while `twig-vm`
        // ran the program. See [`dyn_compare`].
        for sym in ["<", "<=", ">", ">="] {
            reg.register(sym, move |args| dyn_compare(sym, args));
        }

        // `pair?` (#18463) — a dynamic *predicate* the structural lowering
        // leaves as a `call_builtin`: `null?` becomes the `is_null` opcode and
        // `car`/`cdr`/`cons` become field ops, but `pair?` survives, and a Twig
        // record's `<name>?` predicate is compiled to it. See [`dyn_is_pair`].
        // `number?`, `symbol?`, `not` and `list?` survive too but stay
        // unregistered: on this VM a symbol and nil are both `Int`s, so each
        // needs a value-representation decision, not just a handler.
        // `boolean?`, which the shared lowering does not handle for any
        // backend, waits with them.
        reg.register("pair?", dyn_is_pair);

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

    /// Every registered name, sorted — so an embedder (or a test) can check
    /// what a program may `call_builtin`.
    ///
    /// ```
    /// use vm_core::builtins::BuiltinRegistry;
    /// let reg = BuiltinRegistry::new();
    /// let names = reg.names();
    /// assert!(names.contains(&"putchar"));
    /// assert!(names.windows(2).all(|w| w[0] < w[1]));
    /// ```
    pub fn names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.handlers.keys().map(String::as_str).collect();
        names.sort_unstable();
        names
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
        assert_eq!(
            reg.call("equal?", &[Value::Int(1 << 29), Value::Int(1 << 29)]).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            reg.call("equal?", &[Value::Int(1 << 29), Value::Int((1 << 29) + 1)]).unwrap(),
            Value::Bool(false)
        );
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
        assert!(reg.call("equal?", &[Value::Int(1)]).is_err(), "wrong arity");
        assert!(
            matches!(reg.call("*", &[Value::Int(1), Value::Null]), Err(VMError::TypeError { .. })),
            "non-numeric operand is a clean type error"
        );
    }

    // ── #18463: dynamic comparisons and `pair?` ─────────────────────────────
    //
    // Each of these failed before the fix with
    // `UnknownOpcode("builtin \"<\"")` (or `"pair?"`): the names were simply
    // not in the default registry.

    /// The four ordering builtins exist by default — the exact gap #18463
    /// reported, pinned by name so a refactor of `new()` cannot drop one.
    #[test]
    fn dyn_compare_builtins_are_registered_by_default() {
        let reg = BuiltinRegistry::new();
        for name in ["<", "<=", ">", ">=", "=", "pair?"] {
            assert!(reg.has(name), "{name} must be a default builtin");
        }
    }

    /// Full truth table on integers, including the cases a sign or
    /// off-by-one bug would get wrong: equal operands (strict vs non-strict)
    /// and a negative left operand (`-5 < 0` — #18463's own program).
    #[test]
    fn dyn_compare_integer_truth_table() {
        let reg = BuiltinRegistry::new();
        let cmp = |op: &str, a: i64, b: i64| reg.call(op, &[Value::Int(a), Value::Int(b)]).unwrap();
        //             a    b     <      <=     >      >=
        let table: &[(i64, i64, bool, bool, bool, bool)] = &[
            (-5, 0, true, true, false, false),
            (0, -5, false, false, true, true),
            (3, 3, false, true, false, true),
            (i64::MIN, i64::MAX, true, true, false, false),
            (1 << 40, (1 << 40) + 1, true, true, false, false),
        ];
        for &(a, b, lt, le, gt, ge) in table {
            assert_eq!(cmp("<", a, b), Value::Bool(lt), "{a} < {b}");
            assert_eq!(cmp("<=", a, b), Value::Bool(le), "{a} <= {b}");
            assert_eq!(cmp(">", a, b), Value::Bool(gt), "{a} > {b}");
            assert_eq!(cmp(">=", a, b), Value::Bool(ge), "{a} >= {b}");
        }
    }

    /// Two floats compare as IEEE doubles, like `+`/`-`/`*` accept two
    /// floats; a NaN makes every ordering false.
    #[test]
    fn dyn_compare_floats() {
        let reg = BuiltinRegistry::new();
        let f = |op: &str, a: f64, b: f64| reg.call(op, &[Value::Float(a), Value::Float(b)]).unwrap();
        assert_eq!(f("<", -1.5, 0.0), Value::Bool(true));
        assert_eq!(f(">=", 2.0, 2.0), Value::Bool(true));
        assert_eq!(f(">", 2.0, 2.0), Value::Bool(false));
        for op in ["<", "<=", ">", ">="] {
            assert_eq!(f(op, f64::NAN, 1.0), Value::Bool(false), "NaN {op} 1.0");
        }
    }

    /// Non-numbers are a `TypeError`, as on `twig-vm` ("`<` expects
    /// integers"). A boolean is not a number here even though
    /// `Value::as_i64` would read it as 0/1, and a mixed int/float pair is
    /// refused exactly as dynamic `+` refuses it.
    #[test]
    fn dyn_compare_rejects_non_numbers_and_bad_arity() {
        let reg = BuiltinRegistry::new();
        let bad: &[(Value, Value)] = &[
            (Value::Bool(true), Value::Int(2)),
            (Value::Int(1), Value::Str("a".into())),
            (Value::Null, Value::Int(0)),
            (Value::Int(1), Value::Float(2.0)),
            (Value::HeapRef(gc_core::HeapRef::new(0x1000)), Value::Int(0)),
        ];
        for op in ["<", "<=", ">", ">="] {
            for (a, b) in bad {
                let err = reg.call(op, &[a.clone(), b.clone()]).unwrap_err();
                match err {
                    VMError::TypeError { context, .. } => assert!(context.contains(op), "{context}"),
                    other => panic!("{op} {a:?} {b:?}: expected TypeError, got {other:?}"),
                }
            }
            assert!(reg.call(op, &[Value::Int(1)]).is_err(), "{op} with one argument");
            assert!(
                reg.call(op, &[Value::Int(1), Value::Int(2), Value::Int(3)]).is_err(),
                "{op} has no chained form"
            );
        }
    }

    /// `pair?`: a live heap cell is a pair; nil (both of its shapes on this
    /// VM) and every atom are not.
    #[test]
    fn pair_p_distinguishes_cells_from_nil_and_atoms() {
        let reg = BuiltinRegistry::new();
        let pair = |v: Value| reg.call("pair?", &[v]).unwrap();
        assert_eq!(pair(Value::HeapRef(gc_core::HeapRef::new(0x1000))), Value::Bool(true));
        assert_eq!(pair(Value::HeapRef(gc_core::HeapRef::NULL)), Value::Bool(false), "null ref is nil");
        assert_eq!(pair(Value::Int(0)), Value::Bool(false), "`const 0 : ref<LispyPair>` nil");
        for atom in [Value::Int(42), Value::Bool(true), Value::Str("s".into()), Value::Float(1.5), Value::Null] {
            assert_eq!(pair(atom.clone()), Value::Bool(false), "{atom:?}");
        }
        assert!(reg.call("pair?", &[]).is_err(), "pair? needs its argument");
        assert!(reg.call("pair?", &[Value::Int(1), Value::Int(2)]).is_err());
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
    fn lang83_output_builtins_are_registered_by_default() {
        let reg = BuiltinRegistry::new();
        assert!(reg.has("print_i64"));
        assert!(reg.has("putchar"));
        assert!(reg.has("write_i64"));
        // Non-integers are clean type errors, never a panic or a silent 0.
        for name in ["print_i64", "putchar", "write_i64"] {
            let err = reg.call(name, &[Value::Str("x".into())]).unwrap_err();
            assert!(matches!(err, VMError::TypeError { .. }), "{name}");
            let err = reg.call(name, &[]).unwrap_err();
            assert!(matches!(err, VMError::TypeError { .. }), "{name}");
        }
        // The happy path writes to the process stdout and returns Null.
        assert_eq!(reg.call("print_i64", &[Value::Int(7)]).unwrap(), Value::Null);
        assert_eq!(reg.call("putchar", &[Value::Int(0x10A)]).unwrap(), Value::Null);
        assert_eq!(reg.call("write_i64", &[Value::Int(i64::MIN)]).unwrap(), Value::Null);
    }

    /// LANG83 PR7e: the float writers are gone, and no default builtin
    /// names a source language.
    #[test]
    fn default_builtins_name_no_language() {
        let reg = BuiltinRegistry::new();
        for name in ["write_f64_py", "write_f64_js"] {
            assert!(!reg.has(name), "{name} was removed in LANG83 PR7e");
            assert!(matches!(reg.call(name, &[Value::Float(1.5)]), Err(VMError::UnknownOpcode(_))));
        }
        let names = reg.names();
        assert!(names.len() >= 7, "{names:?}");
        for name in names {
            if let Some(why) = interpreter_ir::op_registry::language_name_violation(name) {
                panic!("vm-core default builtin: {why}");
            }
        }
    }

    #[test]
    fn print_str_requires_a_string() {
        let reg = BuiltinRegistry::new();
        let err = reg.call("print_str", &[Value::Int(1)]).unwrap_err();
        assert!(matches!(err, VMError::TypeError { .. }));
    }
}
