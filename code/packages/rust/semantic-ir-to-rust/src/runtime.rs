//! Inlined Rust runtime helpers.
//!
//! The Rust backend produces self-contained output: every generated
//! `.rs` file embeds the runtime helpers it needs.  This module
//! supplies that runtime as a single string constant pasted into
//! every artifact.
//!
//! Per SIR13, the runtime is byte-identical across modules.
//! Dead-code elimination is the responsibility of `rustc` /
//! `cargo` (release builds with LTO strip unused helpers).
//!
//! Style notes:
//!
//! - The runtime lives inside a `mod __sir` inner module so it
//!   never collides with user names.
//! - `Rc` is used (not `Arc`) — single-threaded by design.
//! - No external crate dependencies.

/// The full inlined runtime.  Always emitted verbatim.
pub const RUNTIME: &str = r##"mod __sir {
    //! Runtime support — value model, builtins, helpers.
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    // ── value model ───────────────────────────────────────────────
    #[derive(Clone)]
    pub enum Value {
        Int(i64),
        // SIR16 floats.  Kept distinct from `Int` so the value model
        // never silently coerces — arithmetic promotes to `Float` only
        // when an operand is already a `Float` (see `any_float`).
        Float(f64),
        Bool(bool),
        Nil,
        Sym(Rc<str>),
        Str(Rc<str>),
        Pair(Rc<Pair>),
        Closure(Rc<Closure>),
    }

    pub struct Pair {
        pub car: Value,
        pub cdr: Value,
    }

    pub struct Closure {
        pub fun: Box<dyn Fn(Vec<Value>) -> Value + 'static>,
    }

    // ── symbol interning ──────────────────────────────────────────
    thread_local! {
        static SYMBOL_TABLE: RefCell<HashMap<String, Rc<str>>> =
            RefCell::new(HashMap::new());
    }

    pub fn intern(name: &str) -> Value {
        SYMBOL_TABLE.with(|t| {
            let mut t = t.borrow_mut();
            if let Some(s) = t.get(name) {
                return Value::Sym(s.clone());
            }
            let s: Rc<str> = Rc::from(name);
            t.insert(name.to_string(), s.clone());
            Value::Sym(s)
        })
    }

    // ── global storage ────────────────────────────────────────────
    thread_local! {
        static GLOBALS: RefCell<HashMap<String, Value>> =
            RefCell::new(HashMap::new());
    }

    pub fn global_set(name: &Value, value: Value) -> Value {
        let key = sym_or_string(name);
        GLOBALS.with(|g| g.borrow_mut().insert(key, value.clone()));
        value
    }

    pub fn global_get(name: &Value) -> Value {
        let key = sym_or_string(name);
        GLOBALS.with(|g| {
            g.borrow()
                .get(&key)
                .cloned()
                .unwrap_or_else(|| panic!("undefined global: {}", key))
        })
    }

    pub fn global_get_static(name: &str) -> Value {
        GLOBALS.with(|g| {
            g.borrow()
                .get(name)
                .cloned()
                .unwrap_or_else(|| panic!("undefined global: {}", name))
        })
    }

    fn sym_or_string(v: &Value) -> String {
        match v {
            Value::Sym(s) => s.to_string(),
            Value::Str(s) => s.to_string(),
            other => format(other),
        }
    }

    // ── closure application ───────────────────────────────────────
    pub fn apply_closure(c: &Value, args: Vec<Value>) -> Value {
        match c {
            Value::Closure(cl) => (cl.fun)(args),
            _ => panic!("apply on non-closure"),
        }
    }

    // ── truthiness ────────────────────────────────────────────────
    pub fn truthy(v: &Value) -> bool {
        !matches!(v, Value::Bool(false) | Value::Nil)
    }

    // ── arithmetic builtins (variadic) ────────────────────────────
    //
    // Numeric tower (SIR16): the helpers stay on the integer path while
    // every operand is an `Int`, preserving exact i64 semantics
    // (including `times`' wrapping).  The moment *any* operand is a
    // `Float`, the whole fold promotes to f64 — matching the
    // "int op float ⇒ float" rule of Python/Ruby/JS.
    pub fn plus(args: Vec<Value>) -> Value {
        if any_float(&args) {
            let mut total = 0.0f64;
            for a in &args {
                total += as_f64(a);
            }
            return Value::Float(total);
        }
        let mut total: i64 = 0;
        for a in args {
            total += as_i64(&a);
        }
        Value::Int(total)
    }

    pub fn minus(args: Vec<Value>) -> Value {
        if args.is_empty() {
            return Value::Int(0);
        }
        if any_float(&args) {
            if args.len() == 1 {
                return Value::Float(-as_f64(&args[0]));
            }
            let mut acc = as_f64(&args[0]);
            for a in &args[1..] {
                acc -= as_f64(a);
            }
            return Value::Float(acc);
        }
        if args.len() == 1 {
            return Value::Int(-as_i64(&args[0]));
        }
        let mut acc = as_i64(&args[0]);
        for a in &args[1..] {
            acc -= as_i64(a);
        }
        Value::Int(acc)
    }

    pub fn times(args: Vec<Value>) -> Value {
        if any_float(&args) {
            let mut acc = 1.0f64;
            for a in &args {
                acc *= as_f64(a);
            }
            return Value::Float(acc);
        }
        let mut acc: i64 = 1;
        for a in args {
            acc = acc.wrapping_mul(as_i64(&a));
        }
        Value::Int(acc)
    }

    pub fn divide(args: Vec<Value>) -> Value {
        if args.is_empty() {
            return Value::Int(0);
        }
        if any_float(&args) {
            // Float division follows IEEE-754: `1.0 / 0.0` is `inf`
            // rather than a panic.  Only the all-integer path keeps the
            // historical divide-by-zero panic.
            let mut acc = as_f64(&args[0]);
            for a in &args[1..] {
                acc /= as_f64(a);
            }
            return Value::Float(acc);
        }
        let mut acc = as_i64(&args[0]);
        for a in &args[1..] {
            let d = as_i64(a);
            if d == 0 {
                panic!("division by zero");
            }
            acc /= d;
        }
        Value::Int(acc)
    }

    // ── comparison ────────────────────────────────────────────────
    pub fn eq(a: Value, b: Value) -> Value {
        Value::Bool(value_eq(&a, &b))
    }
    pub fn lt(a: Value, b: Value) -> Value {
        Value::Bool(num_lt(&a, &b))
    }
    pub fn gt(a: Value, b: Value) -> Value {
        Value::Bool(num_lt(&b, &a))
    }

    // Ordered numeric comparison.  Both-int compares as i64 (no
    // precision loss for large magnitudes); any float operand lifts the
    // comparison into f64.  `gt` is defined as `num_lt` with operands
    // swapped, so the two share one source of truth.
    fn num_lt(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => x < y,
            _ => as_f64(a) < as_f64(b),
        }
    }

    // ── pair ops ──────────────────────────────────────────────────
    pub fn cons(a: Value, b: Value) -> Value {
        Value::Pair(Rc::new(Pair { car: a, cdr: b }))
    }
    pub fn car(a: Value) -> Value {
        match a {
            Value::Pair(p) => p.car.clone(),
            _ => panic!("car on non-pair"),
        }
    }
    pub fn cdr(a: Value) -> Value {
        match a {
            Value::Pair(p) => p.cdr.clone(),
            _ => panic!("cdr on non-pair"),
        }
    }

    // ── predicates ────────────────────────────────────────────────
    pub fn is_null(a: Value) -> Value { Value::Bool(matches!(a, Value::Nil)) }
    pub fn is_pair(a: Value) -> Value { Value::Bool(matches!(a, Value::Pair(_))) }
    // `number?` is true for both integers and floats — the predicate
    // names the numeric tower, not a single representation.
    pub fn is_number(a: Value) -> Value { Value::Bool(matches!(a, Value::Int(_) | Value::Float(_))) }
    pub fn is_symbol(a: Value) -> Value { Value::Bool(matches!(a, Value::Sym(_))) }

    // ── print ─────────────────────────────────────────────────────
    pub fn print(a: Value) -> Value {
        println!("{}", format(&a));
        Value::Nil
    }

    // ── format ────────────────────────────────────────────────────
    pub fn format(v: &Value) -> String {
        match v {
            Value::Int(n) => n.to_string(),
            // `{:?}` keeps a trailing `.0` on integral floats (`3.0`,
            // not `3`) so the printed form is unambiguously a float —
            // matching how Python/Ruby render `3.0`.  Non-finite values
            // print as `NaN` / `inf` / `-inf`.
            Value::Float(x) => format_float(*x),
            Value::Bool(true) => "#t".to_string(),
            Value::Bool(false) => "#f".to_string(),
            Value::Nil => "nil".to_string(),
            Value::Sym(s) => s.to_string(),
            Value::Str(s) => s.to_string(),
            Value::Pair(p) => format_pair(p),
            Value::Closure(_) => "<closure>".to_string(),
        }
    }

    fn format_float(x: f64) -> String {
        if x.is_nan() {
            "NaN".to_string()
        } else if x.is_infinite() {
            if x > 0.0 { "inf".to_string() } else { "-inf".to_string() }
        } else {
            format!("{:?}", x)
        }
    }

    fn format_pair(p: &Pair) -> String {
        let mut out = String::new();
        out.push('(');
        out.push_str(&format(&p.car));
        let mut rest = p.cdr.clone();
        loop {
            match rest {
                Value::Pair(inner) => {
                    out.push(' ');
                    out.push_str(&format(&inner.car));
                    rest = inner.cdr.clone();
                }
                Value::Nil => break,
                other => {
                    out.push_str(" . ");
                    out.push_str(&format(&other));
                    break;
                }
            }
        }
        out.push(')');
        out
    }

    // ── helpers ───────────────────────────────────────────────────
    fn as_i64(v: &Value) -> i64 {
        match v {
            Value::Int(n) => *n,
            other => panic!("expected int, got {}", format(other)),
        }
    }

    // Coerce any number to f64 for the promoted arithmetic/comparison
    // paths.  Integers widen losslessly for values within ±2^53; beyond
    // that the all-integer fast paths above keep exactness, so this
    // widening only runs once a float is genuinely in play.
    fn as_f64(v: &Value) -> f64 {
        match v {
            Value::Int(n) => *n as f64,
            Value::Float(x) => *x,
            other => panic!("expected number, got {}", format(other)),
        }
    }

    fn any_float(args: &[Value]) -> bool {
        args.iter().any(|v| matches!(v, Value::Float(_)))
    }

    fn value_eq(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => x == y,
            // Cross-representation numeric equality (`1 == 1.0`) holds,
            // mirroring dynamic-language `==`.  Float/Float uses IEEE
            // equality, so `NaN == NaN` is correctly `false`.
            (Value::Float(x), Value::Float(y)) => x == y,
            (Value::Int(x), Value::Float(y)) => (*x as f64) == *y,
            (Value::Float(x), Value::Int(y)) => *x == (*y as f64),
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::Nil, Value::Nil) => true,
            (Value::Sym(x), Value::Sym(y)) => x == y,
            (Value::Str(x), Value::Str(y)) => **x == **y,
            (Value::Pair(x), Value::Pair(y)) => {
                value_eq(&x.car, &y.car) && value_eq(&x.cdr, &y.cdr)
            }
            _ => false,
        }
    }

    // ── builtin-by-name dispatch (for VarRef Builtin or forward-compat) ─
    pub fn builtin_closure(name: &str) -> Value {
        let n = name.to_string();
        Value::Closure(Rc::new(Closure {
            fun: Box::new(move |args: Vec<Value>| call_builtin_by_name(&n, args)),
        }))
    }

    pub fn call_builtin_by_name(name: &str, args: Vec<Value>) -> Value {
        match name {
            "+" => plus(args),
            "-" => minus(args),
            "*" => times(args),
            "/" => divide(args),
            "=" => {
                let mut it = args.into_iter();
                eq(it.next().unwrap_or(Value::Nil), it.next().unwrap_or(Value::Nil))
            }
            "<" => {
                let mut it = args.into_iter();
                lt(it.next().unwrap_or(Value::Nil), it.next().unwrap_or(Value::Nil))
            }
            ">" => {
                let mut it = args.into_iter();
                gt(it.next().unwrap_or(Value::Nil), it.next().unwrap_or(Value::Nil))
            }
            "cons" => {
                let mut it = args.into_iter();
                cons(it.next().unwrap_or(Value::Nil), it.next().unwrap_or(Value::Nil))
            }
            "car" => car(args.into_iter().next().unwrap_or(Value::Nil)),
            "cdr" => cdr(args.into_iter().next().unwrap_or(Value::Nil)),
            "null?" => is_null(args.into_iter().next().unwrap_or(Value::Nil)),
            "pair?" => is_pair(args.into_iter().next().unwrap_or(Value::Nil)),
            "number?" => is_number(args.into_iter().next().unwrap_or(Value::Nil)),
            "symbol?" => is_symbol(args.into_iter().next().unwrap_or(Value::Nil)),
            "print" => print(args.into_iter().next().unwrap_or(Value::Nil)),
            "global_set" => {
                let mut it = args.into_iter();
                let key = it.next().unwrap_or(Value::Nil);
                let value = it.next().unwrap_or(Value::Nil);
                global_set(&key, value)
            }
            "global_get" => {
                let key = args.into_iter().next().unwrap_or(Value::Nil);
                global_get(&key)
            }
            other => panic!("unknown builtin: {}", other),
        }
    }
}
"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_non_empty_ends_newline() {
        assert!(!RUNTIME.is_empty());
        assert!(RUNTIME.ends_with('\n'));
    }

    #[test]
    fn runtime_declares_module_and_value() {
        assert!(RUNTIME.contains("mod __sir"));
        assert!(RUNTIME.contains("pub enum Value"));
    }

    #[test]
    fn runtime_declares_float_variant_and_helpers() {
        // SIR16 floats: the value model gains a `Float(f64)` arm, and
        // the numeric helpers gain f64 coercion + a display path.
        assert!(RUNTIME.contains("Float(f64)"));
        assert!(RUNTIME.contains("fn as_f64"));
        assert!(RUNTIME.contains("fn any_float"));
        assert!(RUNTIME.contains("fn format_float"));
        assert!(RUNTIME.contains("fn num_lt"));
    }

    #[test]
    fn runtime_includes_all_builtins() {
        for op in &[
            "plus", "minus", "times", "divide", "eq", "lt", "gt",
            "cons", "car", "cdr", "is_null", "is_pair", "is_number",
            "is_symbol", "print", "global_set", "global_get",
            "apply_closure", "intern", "truthy", "format",
            "call_builtin_by_name", "builtin_closure",
        ] {
            assert!(RUNTIME.contains(op), "runtime missing `{}`", op);
        }
    }

    #[test]
    fn runtime_uses_thread_local_globals() {
        assert!(RUNTIME.contains("thread_local!"));
        assert!(RUNTIME.contains("static GLOBALS"));
        assert!(RUNTIME.contains("static SYMBOL_TABLE"));
    }
}
