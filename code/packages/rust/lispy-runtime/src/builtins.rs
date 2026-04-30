//! # Lispy builtins.
//!
//! These are the host-side handlers the IIR's `call_builtin`
//! opcode dispatches to.  Each is a `fn(&[LispyValue]) ->
//! Result<LispyValue, RuntimeError>` — the [`BuiltinFn<L>`] shape
//! from LANG20 §"The LangBinding trait".
//!
//! ## Builtin set (TW00)
//!
//! | Name | Arity | Returns |
//! |------|------:|---------|
//! | `+` `-` `*` `/` | n-ary (≥1 for `+`/`*`; ≥1 for unary `-`/`/`; binary otherwise) | int |
//! | `=` `<` `>` | binary | bool |
//! | `cons` | 2 | cons cell |
//! | `car` `cdr` | 1 | element of pair |
//! | `null?` | 1 | bool |
//! | `pair?` | 1 | bool |
//! | `number?` | 1 | bool |
//! | `symbol?` | 1 | bool |
//! | `print` | 1 | nil (and writes to stdout) |
//!
//! ## Error semantics
//!
//! Wrong arity → `RuntimeError::TypeError("<name> expects N args")`.
//! Wrong operand type → `RuntimeError::TypeError("<name> expects <kind>")`.
//! Division by zero → `RuntimeError::TypeError("division by zero")`.
//!
//! Real Scheme-style condition systems are out of scope for PR 2;
//! the binding's exception model layers on top once a frontend
//! needs it.

use lang_runtime_core::RuntimeError;

use crate::heap;
use crate::value::LispyValue;

// ---------------------------------------------------------------------------
// Arity / type helpers
// ---------------------------------------------------------------------------

/// Format a "expected N args, got M" error message.
fn arity_error(name: &str, expected: usize, got: usize) -> RuntimeError {
    RuntimeError::TypeError(format!(
        "{name} expects {expected} arg{}, got {got}",
        if expected == 1 { "" } else { "s" }
    ))
}

/// Format a "expected at least N args" error message.
fn arity_at_least_error(name: &str, expected: usize, got: usize) -> RuntimeError {
    RuntimeError::TypeError(format!(
        "{name} expects at least {expected} arg{}, got {got}",
        if expected == 1 { "" } else { "s" }
    ))
}

/// Extract the integer or return a typed error.
fn as_int(name: &str, v: LispyValue) -> Result<i64, RuntimeError> {
    v.as_int().ok_or_else(|| {
        RuntimeError::TypeError(format!("{name} expects integers, got {v}"))
    })
}

/// Box an `i64` result back into a tagged `LispyValue`, returning a
/// `TypeError` if the value is outside the representable
/// 61-bit signed range.  Until bignums land (a future PR), Lispy
/// arithmetic is bounded at ±2⁶⁰ and overflow is a clean error
/// rather than a silent truncation.
fn box_int_checked(name: &str, n: i64) -> Result<LispyValue, RuntimeError> {
    use crate::value::{INT_MAX, INT_MIN};
    if !(INT_MIN..=INT_MAX).contains(&n) {
        Err(RuntimeError::TypeError(format!(
            "{name}: integer overflow (result {n} outside [{INT_MIN}, {INT_MAX}])"
        )))
    } else {
        Ok(LispyValue::int(n))
    }
}

// ---------------------------------------------------------------------------
// Arithmetic — variadic per Scheme convention
// ---------------------------------------------------------------------------

/// `(+ a b c ...)`  — sum, identity 0.  Per Scheme: `(+) == 0`.
/// Overflow returns a `TypeError` rather than silently wrapping.
pub fn add(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    let mut sum: i64 = 0;
    for a in args {
        sum = sum.checked_add(as_int("+", *a)?).ok_or_else(|| {
            RuntimeError::TypeError("+: integer overflow".into())
        })?;
    }
    box_int_checked("+", sum)
}

/// `(- a)` → `-a`; `(- a b c ...)` → `a - b - c - ...`.  Unlike
/// `+`, `(-)` is an arity error in Scheme.  Overflow returns a
/// `TypeError`.
pub fn sub(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.is_empty() {
        return Err(arity_at_least_error("-", 1, 0));
    }
    let first = as_int("-", args[0])?;
    if args.len() == 1 {
        let neg = first.checked_neg().ok_or_else(|| {
            RuntimeError::TypeError("-: integer overflow".into())
        })?;
        return box_int_checked("-", neg);
    }
    let mut acc = first;
    for a in &args[1..] {
        acc = acc.checked_sub(as_int("-", *a)?).ok_or_else(|| {
            RuntimeError::TypeError("-: integer overflow".into())
        })?;
    }
    box_int_checked("-", acc)
}

/// `(* a b c ...)` — product, identity 1.  Per Scheme: `(*) == 1`.
/// Overflow returns a `TypeError`.
pub fn mul(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    let mut prod: i64 = 1;
    for a in args {
        prod = prod.checked_mul(as_int("*", *a)?).ok_or_else(|| {
            RuntimeError::TypeError("*: integer overflow".into())
        })?;
    }
    box_int_checked("*", prod)
}

/// `(/ a)` → `1/a`; `(/ a b c ...)` → `a / b / c / ...`.  Integer
/// division (truncates toward zero); divide-by-zero raises.
/// Overflow (e.g. `(/ INT_MIN -1)` whose mathematical result
/// `2⁶⁰` doesn't fit our 61-bit signed range) raises a
/// `TypeError` rather than silently truncating.
pub fn div(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.is_empty() {
        return Err(arity_at_least_error("/", 1, 0));
    }
    let first = as_int("/", args[0])?;
    if args.len() == 1 {
        if first == 0 {
            return Err(RuntimeError::TypeError("division by zero".into()));
        }
        let q = 1i64.checked_div(first).ok_or_else(|| {
            RuntimeError::TypeError("/: integer overflow".into())
        })?;
        return box_int_checked("/", q);
    }
    let mut acc = first;
    for a in &args[1..] {
        let n = as_int("/", *a)?;
        if n == 0 {
            return Err(RuntimeError::TypeError("division by zero".into()));
        }
        acc = acc.checked_div(n).ok_or_else(|| {
            RuntimeError::TypeError("/: integer overflow".into())
        })?;
    }
    box_int_checked("/", acc)
}

// ---------------------------------------------------------------------------
// Comparisons — strictly binary in PR 2
// ---------------------------------------------------------------------------
//
// Scheme's full semantics treat these as transitive chains
// (`(< 1 2 3 4)` ↔ `(and (< 1 2) (< 2 3) (< 3 4))`).  PR 2 ships
// the binary case only because that's what `compile_apply` emits
// today; the chain form can be added when a frontend needs it
// without changing the trait contract.

/// `(= a b)` — integer equality.
pub fn eq(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 2 {
        return Err(arity_error("=", 2, args.len()));
    }
    let a = as_int("=", args[0])?;
    let b = as_int("=", args[1])?;
    Ok(LispyValue::bool(a == b))
}

/// `(< a b)` — integer less-than.
pub fn lt(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 2 {
        return Err(arity_error("<", 2, args.len()));
    }
    let a = as_int("<", args[0])?;
    let b = as_int("<", args[1])?;
    Ok(LispyValue::bool(a < b))
}

/// `(> a b)` — integer greater-than.
pub fn gt(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 2 {
        return Err(arity_error(">", 2, args.len()));
    }
    let a = as_int(">", args[0])?;
    let b = as_int(">", args[1])?;
    Ok(LispyValue::bool(a > b))
}

// ---------------------------------------------------------------------------
// Cons cells
// ---------------------------------------------------------------------------

/// `(cons car cdr)` — allocate a cons cell.
pub fn cons(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 2 {
        return Err(arity_error("cons", 2, args.len()));
    }
    Ok(heap::alloc_cons(args[0], args[1]))
}

/// `(car p)` — first element of a pair.  Errors if `p` isn't a pair.
pub fn car(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 {
        return Err(arity_error("car", 1, args.len()));
    }
    // SAFETY: builtins are dispatched only via the runtime's
    // BuiltinRegistry on values produced by the runtime — heap
    // tags always reflect real allocations (PR 4 wiring upholds
    // this; tests below exercise the contract).
    unsafe { heap::car(args[0]) }.ok_or_else(|| {
        RuntimeError::TypeError(format!("car expects a pair, got {}", args[0]))
    })
}

/// `(cdr p)` — rest of a pair.  Errors if `p` isn't a pair.
pub fn cdr(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 {
        return Err(arity_error("cdr", 1, args.len()));
    }
    // SAFETY: see `car`.
    unsafe { heap::cdr(args[0]) }.ok_or_else(|| {
        RuntimeError::TypeError(format!("cdr expects a pair, got {}", args[0]))
    })
}

// ---------------------------------------------------------------------------
// Type predicates
// ---------------------------------------------------------------------------

/// `(null? x)` — true iff `x` is `nil`.
pub fn null_p(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 {
        return Err(arity_error("null?", 1, args.len()));
    }
    Ok(LispyValue::bool(args[0].is_nil()))
}

/// `(pair? x)` — true iff `x` is a cons cell.
pub fn pair_p(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 {
        return Err(arity_error("pair?", 1, args.len()));
    }
    // SAFETY: see `car`.
    Ok(LispyValue::bool(unsafe { heap::is_cons(args[0]) }))
}

/// `(number? x)` — true iff `x` is an integer.  (PR 2 has only
/// integers; once flonums land this generalises.)
pub fn number_p(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 {
        return Err(arity_error("number?", 1, args.len()));
    }
    Ok(LispyValue::bool(args[0].is_int()))
}

/// `(symbol? x)` — true iff `x` is an interned symbol.
pub fn symbol_p(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 {
        return Err(arity_error("symbol?", 1, args.len()));
    }
    Ok(LispyValue::bool(args[0].is_symbol()))
}

// ---------------------------------------------------------------------------
// I/O
// ---------------------------------------------------------------------------

/// `(print x)` — write `x` to stdout (followed by a newline) and
/// return `nil`.  Uses [`LispyValue`]'s `Display` for formatting.
pub fn print(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 {
        return Err(arity_error("print", 1, args.len()));
    }
    println!("{}", args[0]);
    Ok(LispyValue::NIL)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intern::intern;

    fn i(n: i64) -> LispyValue {
        LispyValue::int(n)
    }

    // ── + ────────────────────────────────────────────────────────────

    #[test]
    fn add_zero_args_is_zero() {
        assert_eq!(add(&[]).unwrap(), i(0));
    }

    #[test]
    fn add_two_args_sums() {
        assert_eq!(add(&[i(1), i(2)]).unwrap(), i(3));
    }

    #[test]
    fn add_many_args_sums() {
        assert_eq!(add(&[i(1), i(2), i(3), i(4)]).unwrap(), i(10));
    }

    #[test]
    fn add_negative_args() {
        assert_eq!(add(&[i(-1), i(-2)]).unwrap(), i(-3));
    }

    #[test]
    fn add_rejects_non_int() {
        let err = add(&[i(1), LispyValue::TRUE]).unwrap_err();
        assert!(matches!(err, RuntimeError::TypeError(_)));
    }

    // ── - ────────────────────────────────────────────────────────────

    #[test]
    fn sub_zero_args_errors() {
        assert!(matches!(sub(&[]).unwrap_err(), RuntimeError::TypeError(_)));
    }

    #[test]
    fn sub_one_arg_negates() {
        assert_eq!(sub(&[i(7)]).unwrap(), i(-7));
        assert_eq!(sub(&[i(-7)]).unwrap(), i(7));
    }

    #[test]
    fn sub_many_args_left_to_right() {
        assert_eq!(sub(&[i(10), i(2), i(3)]).unwrap(), i(5));
    }

    // ── * ────────────────────────────────────────────────────────────

    #[test]
    fn mul_zero_args_is_one() {
        assert_eq!(mul(&[]).unwrap(), i(1));
    }

    #[test]
    fn mul_args_multiply() {
        assert_eq!(mul(&[i(2), i(3), i(4)]).unwrap(), i(24));
    }

    // ── / ────────────────────────────────────────────────────────────

    #[test]
    fn div_one_arg_inverts() {
        // 1 / 7 = 0 with integer truncation (matching Scheme).
        assert_eq!(div(&[i(7)]).unwrap(), i(0));
        assert_eq!(div(&[i(1)]).unwrap(), i(1));
    }

    #[test]
    fn div_many_args_left_to_right() {
        assert_eq!(div(&[i(100), i(2), i(5)]).unwrap(), i(10));
    }

    #[test]
    fn div_by_zero_errors() {
        let err = div(&[i(7), i(0)]).unwrap_err();
        assert!(matches!(err, RuntimeError::TypeError(s) if s.contains("zero")));
    }

    #[test]
    fn div_int_min_div_neg_one_returns_overflow_error() {
        // Per Finding 3 of the security review: i64::MIN / -1
        // overflows; native `/` panics in debug, aborts in release.
        // checked_div returns None; we surface as TypeError.
        // (Our INT_MIN is -2^60, so INT_MIN / -1 == 2^60 which is
        // > INT_MAX = 2^60 - 1 — outside the tagged-int range.)
        use crate::value::INT_MIN;
        let err = div(&[i(INT_MIN), i(-1)]).unwrap_err();
        assert!(
            matches!(&err, RuntimeError::TypeError(s) if s.contains("overflow")),
            "expected overflow error, got {err:?}",
        );
    }

    #[test]
    fn add_overflow_returns_error() {
        use crate::value::INT_MAX;
        let err = add(&[i(INT_MAX), i(1)]).unwrap_err();
        assert!(matches!(err, RuntimeError::TypeError(s) if s.contains("overflow")));
    }

    #[test]
    fn mul_overflow_returns_error() {
        use crate::value::INT_MAX;
        let err = mul(&[i(INT_MAX), i(2)]).unwrap_err();
        assert!(matches!(err, RuntimeError::TypeError(s) if s.contains("overflow")));
    }

    #[test]
    fn sub_overflow_returns_error() {
        use crate::value::INT_MIN;
        let err = sub(&[i(INT_MIN), i(1)]).unwrap_err();
        assert!(matches!(err, RuntimeError::TypeError(s) if s.contains("overflow")));
    }

    #[test]
    fn div_one_arg_zero_errors() {
        let err = div(&[i(0)]).unwrap_err();
        assert!(matches!(err, RuntimeError::TypeError(s) if s.contains("zero")));
    }

    // ── = < > ───────────────────────────────────────────────────────

    #[test]
    fn eq_returns_bool() {
        assert_eq!(eq(&[i(7), i(7)]).unwrap(), LispyValue::TRUE);
        assert_eq!(eq(&[i(7), i(8)]).unwrap(), LispyValue::FALSE);
    }

    #[test]
    fn lt_returns_bool() {
        assert_eq!(lt(&[i(1), i(2)]).unwrap(), LispyValue::TRUE);
        assert_eq!(lt(&[i(2), i(1)]).unwrap(), LispyValue::FALSE);
        assert_eq!(lt(&[i(1), i(1)]).unwrap(), LispyValue::FALSE);
    }

    #[test]
    fn gt_returns_bool() {
        assert_eq!(gt(&[i(2), i(1)]).unwrap(), LispyValue::TRUE);
        assert_eq!(gt(&[i(1), i(2)]).unwrap(), LispyValue::FALSE);
    }

    #[test]
    fn comparisons_reject_wrong_arity() {
        assert!(eq(&[i(1)]).is_err());
        assert!(lt(&[i(1), i(2), i(3)]).is_err());
    }

    // ── cons / car / cdr ────────────────────────────────────────────

    #[test]
    fn cons_then_car_cdr_round_trips() {
        let pair = cons(&[i(7), i(8)]).unwrap();
        assert_eq!(car(&[pair]).unwrap(), i(7));
        assert_eq!(cdr(&[pair]).unwrap(), i(8));
    }

    #[test]
    fn car_of_non_pair_errors() {
        assert!(car(&[i(7)]).is_err());
        assert!(car(&[LispyValue::NIL]).is_err());
    }

    #[test]
    fn cdr_of_non_pair_errors() {
        assert!(cdr(&[i(7)]).is_err());
    }

    // ── Predicates ──────────────────────────────────────────────────

    #[test]
    fn null_p_only_true_for_nil() {
        assert_eq!(null_p(&[LispyValue::NIL]).unwrap(), LispyValue::TRUE);
        assert_eq!(null_p(&[LispyValue::FALSE]).unwrap(), LispyValue::FALSE);
        assert_eq!(null_p(&[i(0)]).unwrap(), LispyValue::FALSE);
    }

    #[test]
    fn pair_p_true_for_cons() {
        let p = cons(&[i(1), i(2)]).unwrap(); // p is from `cons` so it's safe to inspect
        assert_eq!(pair_p(&[p]).unwrap(), LispyValue::TRUE);
        assert_eq!(pair_p(&[LispyValue::NIL]).unwrap(), LispyValue::FALSE);
        assert_eq!(pair_p(&[i(0)]).unwrap(), LispyValue::FALSE);
    }

    #[test]
    fn number_p_true_for_int() {
        assert_eq!(number_p(&[i(0)]).unwrap(), LispyValue::TRUE);
        assert_eq!(number_p(&[i(-1)]).unwrap(), LispyValue::TRUE);
        assert_eq!(number_p(&[LispyValue::TRUE]).unwrap(), LispyValue::FALSE);
    }

    #[test]
    fn symbol_p_true_for_interned() {
        let s = LispyValue::symbol(intern("foo"));
        assert_eq!(symbol_p(&[s]).unwrap(), LispyValue::TRUE);
        assert_eq!(symbol_p(&[i(0)]).unwrap(), LispyValue::FALSE);
        assert_eq!(symbol_p(&[LispyValue::NIL]).unwrap(), LispyValue::FALSE);
    }

    // ── print ───────────────────────────────────────────────────────

    #[test]
    fn print_returns_nil() {
        // Stdout side effect isn't easily captured in a unit test;
        // we just verify the return value and arity behaviour.
        assert_eq!(print(&[i(7)]).unwrap(), LispyValue::NIL);
        assert!(print(&[]).is_err());
        assert!(print(&[i(1), i(2)]).is_err());
    }
}
