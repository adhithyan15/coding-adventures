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

/// `(<= a b)` — integer less-than-or-equal (LANG52).
pub fn le(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 2 {
        return Err(arity_error("<=", 2, args.len()));
    }
    let a = as_int("<=", args[0])?;
    let b = as_int("<=", args[1])?;
    Ok(LispyValue::bool(a <= b))
}

/// `(>= a b)` — integer greater-than-or-equal (LANG52).
pub fn ge(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 2 {
        return Err(arity_error(">=", 2, args.len()));
    }
    let a = as_int(">=", args[0])?;
    let b = as_int(">=", args[1])?;
    Ok(LispyValue::bool(a >= b))
}

// ---------------------------------------------------------------------------
// Extended integer arithmetic (LANG52)
// ---------------------------------------------------------------------------
//
// Scheme distinguishes three integer division operations with different
// sign conventions for the result:
//
//   `quotient`  — truncates toward zero (same as C's `/`).
//   `remainder` — remainder after `quotient`; sign matches the *dividend*.
//   `modulo`    — remainder after floor division; sign matches the *divisor*.
//
// Truth table for negative inputs (Scheme reference):
//
//   | a  | b  | quotient | remainder | modulo |
//   |----|----|----------|-----------|--------|
//   | 13 |  4 |  3       |  1        |  1     |
//   |-13 |  4 | -3       | -1        |  3     |
//   | 13 | -4 | -3       |  1        | -3     |
//   |-13 | -4 |  3       | -1        | -1     |
//
// Rust's `%` operator has `remainder` semantics (sign matches dividend),
// so `remainder` maps directly to `checked_rem`, and `modulo` adds one
// adjustment step: if the Rust remainder is non-zero *and* its sign
// differs from the divisor's sign, add the divisor.

/// `(quotient a b)` — truncating integer division (LANG52).
///
/// `(quotient 13 4)` → 3;  `(quotient -13 4)` → -3.
/// Errors on division by zero or signed-overflow (INT_MIN / -1).
pub fn quotient(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 2 {
        return Err(arity_error("quotient", 2, args.len()));
    }
    let a = as_int("quotient", args[0])?;
    let b = as_int("quotient", args[1])?;
    if b == 0 {
        return Err(RuntimeError::TypeError("quotient: division by zero".into()));
    }
    let q = a.checked_div(b).ok_or_else(|| {
        RuntimeError::TypeError("quotient: integer overflow".into())
    })?;
    box_int_checked("quotient", q)
}

/// `(remainder a b)` — remainder after `quotient`; sign matches dividend (LANG52).
///
/// `(remainder 13 4)` → 1;  `(remainder -13 4)` → -1.
pub fn remainder(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 2 {
        return Err(arity_error("remainder", 2, args.len()));
    }
    let a = as_int("remainder", args[0])?;
    let b = as_int("remainder", args[1])?;
    if b == 0 {
        return Err(RuntimeError::TypeError("remainder: division by zero".into()));
    }
    let r = a.checked_rem(b).ok_or_else(|| {
        RuntimeError::TypeError("remainder: integer overflow".into())
    })?;
    box_int_checked("remainder", r)
}

/// `(modulo a b)` — Scheme-style floor-division remainder; sign matches divisor (LANG52).
///
/// `(modulo -13 4)` → 3;  `(modulo 13 -4)` → -3.
pub fn modulo(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 2 {
        return Err(arity_error("modulo", 2, args.len()));
    }
    let a = as_int("modulo", args[0])?;
    let b = as_int("modulo", args[1])?;
    if b == 0 {
        return Err(RuntimeError::TypeError("modulo: division by zero".into()));
    }
    // Rust's `%` gives remainder (sign matches dividend).  One adjustment
    // step converts it to Scheme's modulo (sign matches divisor): if the
    // remainder is nonzero and its sign differs from the divisor's, add b.
    let r = a.checked_rem(b).ok_or_else(|| {
        RuntimeError::TypeError("modulo: integer overflow".into())
    })?;
    let m = if r != 0 && (r < 0) != (b < 0) {
        r.checked_add(b).ok_or_else(|| {
            RuntimeError::TypeError("modulo: integer overflow".into())
        })?
    } else {
        r
    };
    box_int_checked("modulo", m)
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

/// `(boolean? x)` — true iff `x` is `#t` or `#f` (LANG52).
pub fn boolean_p(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 {
        return Err(arity_error("boolean?", 1, args.len()));
    }
    Ok(LispyValue::bool(args[0].is_bool()))
}

// ---------------------------------------------------------------------------
// Logic (LANG52)
// ---------------------------------------------------------------------------
//
// `not` is a straightforward predicate: only `#f` and `nil` are falsy in
// Scheme (and Twig inherits that rule — every other value, including `0`,
// is truthy).
//
// `and` / `or` are special forms handled by the *compiler* as short-circuit
// lowerings — they never appear here as builtins.  Only `not` needs to live
// in the runtime because it has no compile-time short-circuit behaviour; it
// is a plain 1-argument function.
//
// `equal?` provides structural equality across all value kinds:
//   * immediates    — bitwise comparison (int, bool, nil, symbol)
//   * cons cells    — recursive structural comparison
//   * strings       — byte-for-byte comparison
//   * closures      — identity (eq? semantics for opaque values)
//
// `equal?` is placed here rather than in binding.rs to avoid a circular
// dependency: binding.rs imports builtins; builtins cannot import binding.
// The implementation is a private `values_equal` helper that mirrors
// `LispyBinding::equal` exactly.

/// Structural equality used by `equal?` and `assoc`.
///
/// Private helper — not a builtin entry point.  Mirrors
/// [`crate::binding::LispyBinding::equal`] without the circular dep.
///
/// # Safety / DoS hardening
///
/// Uses an **explicit work-stack** (iterative, not recursive) to avoid Rust
/// call-stack overflow on arbitrarily-deep nested lists.  A depth cap of
/// 4096 pairs prevents adversarial input from growing the work-stack without
/// bound while still handling any practically reasonable data structure.  On
/// hitting the cap the function returns `false` (safe-conservative).
fn values_equal(a: LispyValue, b: LispyValue) -> bool {
    /// Maximum number of pair comparisons before we give up.
    ///
    /// A deeply-nested list `(cons 1 (cons 1 … nil))` with 4 096 levels
    /// requires exactly 4 096 work-stack entries.  Real Twig data
    /// structures are well below this limit; the cap is a DoS guard, not
    /// a semantic limit on `equal?`.
    const MAX_PAIRS: usize = 4096;

    // The work-stack holds pairs of values that must themselves be equal
    // for the overall comparison to be `true`.  We pop one pair per
    // iteration, check for fast-path equality, and push sub-pairs for
    // cons cells.
    let mut work: Vec<(LispyValue, LispyValue)> = Vec::with_capacity(16);
    work.push((a, b));

    while let Some((a, b)) = work.pop() {
        // Fast path: bitwise equality covers all immediates (int, bool,
        // nil, symbol) and pointer-equal heap objects.
        if a.bits() == b.bits() {
            continue;
        }

        // Depth / DoS guard.
        if work.len() >= MAX_PAIRS {
            return false;
        }

        // SAFETY: values in the work-stack originate from the runtime's
        // value space — heap tags always reflect real, live allocations.
        unsafe {
            if heap::is_cons(a) && heap::is_cons(b) {
                // Structural: push both car/car and cdr/cdr pairs.
                let ca = heap::car(a); let da = heap::cdr(a);
                let cb = heap::car(b); let db = heap::cdr(b);
                match (ca, da, cb, db) {
                    (Some(ca), Some(da), Some(cb), Some(db)) => {
                        work.push((ca, cb));
                        work.push((da, db));
                        continue;
                    }
                    _ => return false,
                }
            }
            if heap::is_string(a) && heap::is_string(b) {
                // Byte-for-byte string equality.
                let ab = heap::string_bytes(a).unwrap_or(&[]);
                let bb = heap::string_bytes(b).unwrap_or(&[]);
                if ab == bb { continue; } else { return false; }
            }
        }
        // Different types — not equal.
        return false;
    }
    // All pairs in the work-stack compared equal.
    true
}

/// `(not x)` — logical negation; returns `#t` iff `x` is falsy (LANG52).
///
/// Scheme falsy values: `#f` and `nil`.  Everything else — including `0`
/// and the empty string — is truthy.
pub fn not(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 {
        return Err(arity_error("not", 1, args.len()));
    }
    Ok(LispyValue::bool(!args[0].is_truthy()))
}

/// `(equal? a b)` — structural equality (LANG52).
///
/// Integers, booleans, nil, and symbols compare by value.  Cons cells
/// recurse on car/cdr.  Strings compare byte-for-byte.  Closures compare
/// by identity (pointer equality, matching R7RS for opaque values).
pub fn equal_p(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 2 {
        return Err(arity_error("equal?", 2, args.len()));
    }
    Ok(LispyValue::bool(values_equal(args[0], args[1])))
}

// ---------------------------------------------------------------------------
// List operations (LANG52)
// ---------------------------------------------------------------------------
//
// These operations build on the cons-cell primitives (`cons`, `car`, `cdr`)
// already in this file.  Higher-order operations (`map`, `filter`,
// `fold-left`) require calling back into the VM interpreter and are deferred
// to LANG53 or a stdlib.tw once the self-hosted compiler exists.
//
// | Builtin     | Arity | Description                                      |
// |-------------|------:|--------------------------------------------------|
// | `list`      | n-ary | Build a proper list from positional args         |
// | `list?`     |     1 | Proper-list predicate (nil-terminated)           |
// | `length`    |     1 | Number of elements in a proper list              |
// | `append`    |     2 | Concatenate two proper lists                     |
// | `reverse`   |     1 | Reverse a proper list                            |
// | `list-ref`  |     2 | 0-indexed element access                         |
// | `assoc`     |     2 | Alist lookup; uses `equal?` for key comparison   |

/// `(list a b c ...)` — construct a proper list from positional args (LANG52).
///
/// `(list 1 2 3)` → `(1 2 3)` (= `(cons 1 (cons 2 (cons 3 nil)))`).
/// `(list)` → `nil`.
pub fn list(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    // Build the list from the right so each `alloc_cons` prepends one cell.
    let mut result = LispyValue::NIL;
    for a in args.iter().rev() {
        result = heap::alloc_cons(*a, result);
    }
    Ok(result)
}

/// `(list? x)` — `#t` iff `x` is a proper (nil-terminated) list (LANG52).
///
/// `(list? nil)` → `#t`.  `(list? (cons 1 2))` → `#f` (improper list).
/// Uses an iterative walk to avoid stack overflow on long lists.
pub fn list_p(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 {
        return Err(arity_error("list?", 1, args.len()));
    }
    let mut cur = args[0];
    loop {
        if cur.is_nil() {
            return Ok(LispyValue::TRUE);
        }
        // SAFETY: see `car`.
        if !unsafe { heap::is_cons(cur) } {
            return Ok(LispyValue::FALSE);
        }
        cur = unsafe { heap::cdr(cur) }.unwrap_or(LispyValue::NIL);
    }
}

/// `(length lst)` — number of elements in proper list `lst` (LANG52).
///
/// Errors if `lst` is not a proper list (e.g. a dotted pair or non-list).
pub fn length(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 {
        return Err(arity_error("length", 1, args.len()));
    }
    let mut cur = args[0];
    let mut n: i64 = 0;
    loop {
        if cur.is_nil() {
            return box_int_checked("length", n);
        }
        // SAFETY: see `car`.
        if !unsafe { heap::is_cons(cur) } {
            return Err(RuntimeError::TypeError(
                "length: not a proper list".into(),
            ));
        }
        cur = unsafe { heap::cdr(cur) }.unwrap_or(LispyValue::NIL);
        n += 1;
    }
}

/// `(append lst1 lst2)` — concatenate two proper lists (LANG52).
///
/// The result shares the structure of `lst2`; `lst1` elements are copied
/// (each gets a fresh cons cell).  Errors if `lst1` is not a proper list.
pub fn append(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 2 {
        return Err(arity_error("append", 2, args.len()));
    }
    // Collect the elements of lst1 into a Vec so we can prepend them to
    // lst2 in reverse order.
    let mut items: Vec<LispyValue> = Vec::new();
    let mut cur = args[0];
    loop {
        if cur.is_nil() {
            break;
        }
        // SAFETY: see `car`.
        if !unsafe { heap::is_cons(cur) } {
            return Err(RuntimeError::TypeError(
                "append: first argument is not a proper list".into(),
            ));
        }
        unsafe {
            items.push(heap::car(cur).unwrap_or(LispyValue::NIL));
            cur = heap::cdr(cur).unwrap_or(LispyValue::NIL);
        }
    }
    let mut result = args[1];
    for item in items.iter().rev() {
        result = heap::alloc_cons(*item, result);
    }
    Ok(result)
}

/// `(reverse lst)` — return `lst` with elements in reverse order (LANG52).
///
/// `(reverse '(1 2 3))` → `(3 2 1)`.  Errors if `lst` is not a proper list.
pub fn reverse(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 {
        return Err(arity_error("reverse", 1, args.len()));
    }
    let mut result = LispyValue::NIL;
    let mut cur = args[0];
    loop {
        if cur.is_nil() {
            return Ok(result);
        }
        // SAFETY: see `car`.
        if !unsafe { heap::is_cons(cur) } {
            return Err(RuntimeError::TypeError(
                "reverse: not a proper list".into(),
            ));
        }
        unsafe {
            result = heap::alloc_cons(heap::car(cur).unwrap_or(LispyValue::NIL), result);
            cur = heap::cdr(cur).unwrap_or(LispyValue::NIL);
        }
    }
}

/// `(list-ref lst i)` — return the element at 0-based index `i` (LANG52).
///
/// `(list-ref '(a b c) 1)` → `b`.  Errors if `i` is out of bounds.
pub fn list_ref(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 2 {
        return Err(arity_error("list-ref", 2, args.len()));
    }
    let idx = as_int("list-ref", args[1])?;
    if idx < 0 {
        return Err(RuntimeError::TypeError(format!(
            "list-ref: index {idx} is negative"
        )));
    }
    let mut cur = args[0];
    let mut remaining = idx;
    loop {
        // SAFETY: see `car`.
        if !unsafe { heap::is_cons(cur) } {
            return Err(RuntimeError::TypeError(format!(
                "list-ref: index {idx} out of bounds"
            )));
        }
        if remaining == 0 {
            return Ok(unsafe { heap::car(cur) }.unwrap_or(LispyValue::NIL));
        }
        cur = unsafe { heap::cdr(cur) }.unwrap_or(LispyValue::NIL);
        remaining -= 1;
    }
}

/// `(assoc key alist)` — find the first pair in `alist` whose car is
/// `equal?` to `key`; return the pair, or `#f` if not found (LANG52).
///
/// `(assoc 2 '((1 . a) (2 . b) (3 . c)))` → `(2 . b)`.
pub fn assoc(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 2 {
        return Err(arity_error("assoc", 2, args.len()));
    }
    let key = args[0];
    let mut cur = args[1];
    loop {
        if cur.is_nil() {
            return Ok(LispyValue::FALSE);
        }
        // SAFETY: see `car`.
        if !unsafe { heap::is_cons(cur) } {
            return Err(RuntimeError::TypeError(
                "assoc: second argument is not a proper list".into(),
            ));
        }
        let pair = unsafe { heap::car(cur) }.unwrap_or(LispyValue::NIL);
        // Each element of the alist must itself be a pair.
        if unsafe { heap::is_cons(pair) } {
            let pair_key = unsafe { heap::car(pair) }.unwrap_or(LispyValue::NIL);
            if values_equal(key, pair_key) {
                return Ok(pair);
            }
        }
        cur = unsafe { heap::cdr(cur) }.unwrap_or(LispyValue::NIL);
    }
}

// ---------------------------------------------------------------------------
// Symbol operations (LANG52)
// ---------------------------------------------------------------------------

/// `(symbol-append sym1 sym2)` — intern the concatenation of both symbols'
/// names as a new symbol (LANG52).
///
/// `(symbol-append 'foo 'bar)` → `foobar` (as a symbol).
/// Useful in macro-expansion contexts where generated symbol names are
/// built from component names.
pub fn symbol_append(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 2 {
        return Err(arity_error("symbol-append", 2, args.len()));
    }
    let id1 = args[0].as_symbol().ok_or_else(|| {
        RuntimeError::TypeError(format!(
            "symbol-append: first arg must be a symbol, got {}",
            args[0]
        ))
    })?;
    let id2 = args[1].as_symbol().ok_or_else(|| {
        RuntimeError::TypeError(format!(
            "symbol-append: second arg must be a symbol, got {}",
            args[1]
        ))
    })?;
    let name1 = crate::intern::name_of(id1).ok_or_else(|| {
        RuntimeError::TypeError(format!("symbol-append: unknown symbol id {id1:?}"))
    })?;
    let name2 = crate::intern::name_of(id2).ok_or_else(|| {
        RuntimeError::TypeError(format!("symbol-append: unknown symbol id {id2:?}"))
    })?;
    let combined = format!("{name1}{name2}");
    let id = crate::intern::intern(&combined);
    Ok(LispyValue::symbol(id))
}

// ---------------------------------------------------------------------------
// Infrastructure builtins (compiler-emitted, not user-callable)
// ---------------------------------------------------------------------------
//
// twig-ir-compiler (and any other Lispy frontend) emits a few
// "infrastructure" builtin calls that aren't user-facing — they're
// part of the IR encoding for control-flow plumbing.  We register
// them here so the dispatcher (LANG20 PR 4) resolves them through
// the same `LangBinding::resolve_builtin` path as user builtins.
//
// `_move` — identity copy.  twig-ir-compiler emits this in `(if c
// t e)` lowering: each arm computes a value into a fresh register,
// then `_move` copies it into the shared result register.  The
// indirection exists because a plain `add result, source, 0`
// would coerce booleans to integers in some IIR dialects; `_move`
// is a typed-preserving identity.
//
// `make_nil` — return the nil singleton.  twig-ir-compiler emits
// `call_builtin "make_nil"` everywhere a nil literal is needed
// (since `nil` doesn't have an `Operand::Var(name)` representation
// — it's a heap-derived value in Python, but an immediate in our
// LispyValue scheme).  We return `LispyValue::NIL` directly.

/// `(_move x)` — return `x` unchanged.  Used by twig-ir-compiler
/// for type-preserving register copies in `if` / `let` lowering.
pub fn move_(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 {
        return Err(arity_error("_move", 1, args.len()));
    }
    Ok(args[0])
}

/// `(make_nil)` — return the nil singleton.
pub fn make_nil(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if !args.is_empty() {
        return Err(arity_error("make_nil", 0, args.len()));
    }
    Ok(LispyValue::NIL)
}

/// `(make_symbol name)` — return the symbol named by `name`.
///
/// In our value-space convention, `name` is itself a symbol value
/// (the dispatcher interns string-literal `const` operands and
/// stores them as symbols).  This builtin therefore acts as
/// **identity-with-type-check** — it asserts the arg is a symbol
/// and passes it through.  twig-ir-compiler emits a call to it
/// for every `'foo` quoted-symbol literal in source; the explicit
/// arity check here turns "wrong shape of args" into a clean
/// `RuntimeError` rather than a silent miscompile.
pub fn make_symbol(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 {
        return Err(arity_error("make_symbol", 1, args.len()));
    }
    if !args[0].is_symbol() {
        return Err(RuntimeError::TypeError(format!(
            "make_symbol: expected symbol arg, got {}",
            args[0],
        )));
    }
    Ok(args[0])
}

/// `(make_closure name capture0 capture1 ...)` — allocate a
/// user-fn closure capturing `capture*` over the function named
/// `name`.
///
/// `name` must be a symbol value (the IR compiler emits it via
/// the same `const`-as-symbol path as quoted-symbol literals).
/// At apply time, the dispatcher looks `name` up in the
/// `IIRModule`'s functions table and prepends `captures` to the
/// supplied arguments before recursing.
pub fn make_closure(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.is_empty() {
        return Err(RuntimeError::TypeError(
            "make_closure: expected at least 1 arg (function name)".into(),
        ));
    }
    let name_id = args[0].as_symbol().ok_or_else(|| {
        RuntimeError::TypeError(format!(
            "make_closure: expected symbol name, got {}",
            args[0],
        ))
    })?;
    let captures: Vec<LispyValue> = args[1..].to_vec();
    Ok(crate::heap::alloc_closure(name_id, captures))
}

/// `(make_builtin_closure name)` — allocate a closure wrapping
/// the builtin named `name`.
///
/// Used by twig-ir-compiler when a bare builtin reference (`+`,
/// `cons`, etc.) appears in a higher-order position — we wrap
/// it in a closure-shaped value so it can be passed around like
/// any other callable.  The closure carries no captures; at
/// apply time the dispatcher detects the
/// `CLOSURE_FLAG_BUILTIN` flag and routes through
/// `LispyBinding::resolve_builtin` instead of the user-fn
/// lookup path.
pub fn make_builtin_closure(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 {
        return Err(arity_error("make_builtin_closure", 1, args.len()));
    }
    let name_id = args[0].as_symbol().ok_or_else(|| {
        RuntimeError::TypeError(format!(
            "make_builtin_closure: expected symbol name, got {}",
            args[0],
        ))
    })?;
    Ok(crate::heap::alloc_builtin_closure(name_id))
}

// ---------------------------------------------------------------------------
// String builtins (LANG47)
// ---------------------------------------------------------------------------
//
// Strings are `LangString` heap objects (CLASS_STRING = 3).  Character
// operations work on Unicode scalar values (code points) represented as
// `i64` integers — the same type as Twig integers.  Positions are
// 0-indexed code-point offsets, *not* byte offsets.
//
// Helper: extract bytes from a LispyValue or return a TypeError.

fn as_str_bytes<'a>(name: &str, v: LispyValue) -> Result<&'a [u8], RuntimeError> {
    // SAFETY: v came from the Lispy value space — either a constant produced
    // by alloc_string, or a builtin result.  The `Box::leak` allocator keeps
    // all heap objects live for the process lifetime.
    unsafe {
        heap::string_bytes(v).ok_or_else(|| {
            RuntimeError::TypeError(format!("{name}: expected String, got {v}"))
        })
    }
}

/// Count Unicode code points in a byte slice.  Tries UTF-8 first; falls
/// back to the byte count if the bytes aren't valid UTF-8 (shouldn't happen
/// in well-formed Twig source, but we prefer a useful value over a panic).
fn count_codepoints(bytes: &[u8]) -> usize {
    if let Ok(s) = std::str::from_utf8(bytes) {
        s.chars().count()
    } else {
        bytes.len()
    }
}

/// Collect an iterator of code points up to index `n` and return the
/// byte offset of the n-th code point boundary.  Returns `None` if there
/// are fewer than `n` code points.
fn byte_offset_of_nth_codepoint(bytes: &[u8], n: usize) -> Option<usize> {
    if let Ok(s) = std::str::from_utf8(bytes) {
        let mut offset = 0;
        for (i, ch) in s.char_indices() {
            if offset == n {
                return Some(i);
            }
            offset += 1;
            let _ = ch;
        }
        if offset == n { Some(bytes.len()) } else { None }
    } else {
        // Byte-mode fallback.
        if n <= bytes.len() { Some(n) } else { None }
    }
}

/// `(string? v) → Bool` — type predicate (LANG47).
pub fn string_p(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 {
        return Err(arity_error("string?", 1, args.len()));
    }
    // SAFETY: args[0] is a live LispyValue from the runtime.
    Ok(LispyValue::bool(unsafe { heap::is_string(args[0]) }))
}

/// `(string-length s) → Int` — number of Unicode code points (LANG47).
pub fn string_length(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 {
        return Err(arity_error("string-length", 1, args.len()));
    }
    let bytes = as_str_bytes("string-length", args[0])?;
    let n = count_codepoints(bytes);
    box_int_checked("string-length", n as i64)
}

/// `(string-ref s i) → Int` — return the i-th code point as an integer (LANG47).
///
/// `i` is a 0-indexed code-point position.  Returns a `TypeError` if `i` is
/// out of bounds.
pub fn string_ref(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 2 {
        return Err(arity_error("string-ref", 2, args.len()));
    }
    let bytes = as_str_bytes("string-ref", args[0])?;
    let idx = as_int("string-ref", args[1])?;
    if idx < 0 {
        return Err(RuntimeError::TypeError(format!(
            "string-ref: index {idx} is negative"
        )));
    }
    let idx = idx as usize;
    let cp = if let Ok(s) = std::str::from_utf8(bytes) {
        s.chars()
            .nth(idx)
            .ok_or_else(|| RuntimeError::TypeError(format!(
                "string-ref: index {idx} out of bounds (length {})",
                s.chars().count()
            )))?
            as u32
    } else {
        // Byte-mode fallback.
        *bytes.get(idx).ok_or_else(|| RuntimeError::TypeError(format!(
            "string-ref: index {idx} out of bounds (byte-length {})",
            bytes.len()
        )))? as u32
    };
    Ok(LispyValue::int(cp as i64))
}

/// `(substring s start end) → String` — slice [start, end) by code points (LANG47).
pub fn substring(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 3 {
        return Err(arity_error("substring", 3, args.len()));
    }
    let bytes = as_str_bytes("substring", args[0])?;
    let start = as_int("substring", args[1])?;
    let end   = as_int("substring", args[2])?;
    if start < 0 || end < start {
        return Err(RuntimeError::TypeError(format!(
            "substring: invalid range [{start}, {end})"
        )));
    }
    let start = start as usize;
    let end   = end   as usize;

    let byte_start = byte_offset_of_nth_codepoint(bytes, start)
        .ok_or_else(|| RuntimeError::TypeError(format!(
            "substring: start {start} out of bounds"
        )))?;
    let byte_end = byte_offset_of_nth_codepoint(bytes, end)
        .ok_or_else(|| RuntimeError::TypeError(format!(
            "substring: end {end} out of bounds"
        )))?;
    Ok(heap::alloc_string(&bytes[byte_start..byte_end]))
}

/// `(string-append s1 s2) → String` — concatenate two strings (LANG47).
pub fn string_append(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 2 {
        return Err(arity_error("string-append", 2, args.len()));
    }
    let a = as_str_bytes("string-append", args[0])?;
    let b = as_str_bytes("string-append", args[1])?;
    let mut result = Vec::with_capacity(a.len() + b.len());
    result.extend_from_slice(a);
    result.extend_from_slice(b);
    Ok(heap::alloc_string(&result))
}

/// `(make-string n ch) → String` — build a string of `n` copies of code point `ch` (LANG47).
pub fn make_string(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 2 {
        return Err(arity_error("make-string", 2, args.len()));
    }
    let n  = as_int("make-string", args[0])?;
    let ch = as_int("make-string", args[1])?;
    if n < 0 {
        return Err(RuntimeError::TypeError(format!("make-string: length {n} is negative")));
    }
    let cp = char::from_u32(ch as u32).ok_or_else(|| {
        RuntimeError::TypeError(format!("make-string: {ch} is not a valid Unicode code point"))
    })?;
    let mut s = String::with_capacity(n as usize * cp.len_utf8());
    for _ in 0..n {
        s.push(cp);
    }
    Ok(heap::alloc_string(s.as_bytes()))
}

/// `(string=? s1 s2) → Bool` — byte-for-byte equality (LANG47).
pub fn string_eq_p(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 2 {
        return Err(arity_error("string=?", 2, args.len()));
    }
    let a = as_str_bytes("string=?", args[0])?;
    let b = as_str_bytes("string=?", args[1])?;
    Ok(LispyValue::bool(a == b))
}

/// `(string<? s1 s2) → Bool` — lexicographic less-than (LANG47).
pub fn string_lt_p(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 2 {
        return Err(arity_error("string<?", 2, args.len()));
    }
    let a = as_str_bytes("string<?", args[0])?;
    let b = as_str_bytes("string<?", args[1])?;
    Ok(LispyValue::bool(a < b))
}

/// `(string>? s1 s2) → Bool` — lexicographic greater-than (LANG47).
pub fn string_gt_p(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 2 {
        return Err(arity_error("string>?", 2, args.len()));
    }
    let a = as_str_bytes("string>?", args[0])?;
    let b = as_str_bytes("string>?", args[1])?;
    Ok(LispyValue::bool(a > b))
}

/// `(number->string n) → String` — decimal representation (LANG47).
pub fn number_to_string(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 {
        return Err(arity_error("number->string", 1, args.len()));
    }
    let n = as_int("number->string", args[0])?;
    let s = n.to_string();
    Ok(heap::alloc_string(s.as_bytes()))
}

/// `(string->number s) → Int | #f` — parse decimal integer (LANG47).
///
/// Returns `#f` on parse failure rather than raising an error — this is the
/// R7RS-compatible behaviour (`string->number` is allowed to return `#f`).
pub fn string_to_number(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 {
        return Err(arity_error("string->number", 1, args.len()));
    }
    let bytes = as_str_bytes("string->number", args[0])?;
    let s = std::str::from_utf8(bytes).unwrap_or("");
    match s.trim().parse::<i64>() {
        Ok(n) => box_int_checked("string->number", n),
        Err(_) => Ok(LispyValue::FALSE),
    }
}

/// `(string->symbol s) → Symbol` — intern the string as a symbol (LANG47).
pub fn string_to_symbol(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 {
        return Err(arity_error("string->symbol", 1, args.len()));
    }
    let bytes = as_str_bytes("string->symbol", args[0])?;
    let s = std::str::from_utf8(bytes).map_err(|_| {
        RuntimeError::TypeError("string->symbol: string is not valid UTF-8".into())
    })?;
    let id = crate::intern::intern(s);
    Ok(LispyValue::symbol(id))
}

/// `(symbol->string sym) → String` — look up the symbol's name and return a string (LANG47).
pub fn symbol_to_string(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 {
        return Err(arity_error("symbol->string", 1, args.len()));
    }
    let id = args[0].as_symbol().ok_or_else(|| {
        RuntimeError::TypeError(format!("symbol->string: expected symbol, got {}", args[0]))
    })?;
    let name = crate::intern::name_of(id).ok_or_else(|| {
        RuntimeError::TypeError(format!("symbol->string: unknown symbol id {id:?}"))
    })?;
    Ok(heap::alloc_string(name.as_bytes()))
}

// ---------------------------------------------------------------------------
// Character predicates (LANG47)
//
// In Twig, characters are represented as code-point integers (the result of
// `string-ref`).  All character predicates accept an integer and return a
// boolean.  Only ASCII is covered at this stage.
// ---------------------------------------------------------------------------

/// Extract a code-point integer from a `LispyValue` for char predicates.
fn as_codepoint(name: &str, v: LispyValue) -> Result<u32, RuntimeError> {
    let n = as_int(name, v)?;
    if n < 0 || n > 0x10FFFF {
        return Err(RuntimeError::TypeError(format!(
            "{name}: {n} is not a valid Unicode code point"
        )));
    }
    Ok(n as u32)
}

/// Helper: is `cp` an ASCII alphabetic code point?
#[inline]
fn is_alpha(cp: u32) -> bool {
    (cp >= b'a' as u32 && cp <= b'z' as u32) || (cp >= b'A' as u32 && cp <= b'Z' as u32)
}

/// Helper: is `cp` an ASCII digit code point?
#[inline]
fn is_digit(cp: u32) -> bool {
    cp >= b'0' as u32 && cp <= b'9' as u32
}

/// Helper: is `cp` an ASCII uppercase letter?
#[inline]
fn is_upper(cp: u32) -> bool {
    cp >= b'A' as u32 && cp <= b'Z' as u32
}

/// Helper: is `cp` an ASCII lowercase letter?
#[inline]
fn is_lower(cp: u32) -> bool {
    cp >= b'a' as u32 && cp <= b'z' as u32
}

/// `(char-alphabetic? code) → Bool` — is the code point a letter? (ASCII) (LANG47).
pub fn char_alphabetic_p(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 { return Err(arity_error("char-alphabetic?", 1, args.len())); }
    let cp = as_codepoint("char-alphabetic?", args[0])?;
    Ok(LispyValue::bool(is_alpha(cp)))
}

/// `(char-numeric? code) → Bool` — is the code point an ASCII digit? (LANG47).
pub fn char_numeric_p(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 { return Err(arity_error("char-numeric?", 1, args.len())); }
    let cp = as_codepoint("char-numeric?", args[0])?;
    Ok(LispyValue::bool(is_digit(cp)))
}

/// `(char-whitespace? code) → Bool` — space, tab, newline, CR, FF (LANG47).
pub fn char_whitespace_p(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 { return Err(arity_error("char-whitespace?", 1, args.len())); }
    let cp = as_codepoint("char-whitespace?", args[0])?;
    // R7RS: at least #\space, #\newline, #\tab are whitespace.
    // Common extras: CR (13), FF (12), VT (11).
    Ok(LispyValue::bool(matches!(cp, 9 | 10 | 11 | 12 | 13 | 32)))
}

/// `(char-upper-case? code) → Bool` — is the code point an ASCII uppercase letter? (LANG47).
pub fn char_upper_case_p(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 { return Err(arity_error("char-upper-case?", 1, args.len())); }
    let cp = as_codepoint("char-upper-case?", args[0])?;
    Ok(LispyValue::bool(is_upper(cp)))
}

/// `(char-lower-case? code) → Bool` — is the code point an ASCII lowercase letter? (LANG47).
pub fn char_lower_case_p(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 { return Err(arity_error("char-lower-case?", 1, args.len())); }
    let cp = as_codepoint("char-lower-case?", args[0])?;
    Ok(LispyValue::bool(is_lower(cp)))
}

/// `(char->integer code) → Int` — identity in our encoding (LANG47).
///
/// Provided for R7RS source compatibility: in Twig's encoding, chars *are*
/// code-point integers, so this is a no-op.
pub fn char_to_integer(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 { return Err(arity_error("char->integer", 1, args.len())); }
    let _ = as_codepoint("char->integer", args[0])?;
    Ok(args[0])
}

/// `(integer->char n) → Int` — identity in our encoding (LANG47).
pub fn integer_to_char(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 { return Err(arity_error("integer->char", 1, args.len())); }
    let _ = as_codepoint("integer->char", args[0])?;
    Ok(args[0])
}

/// `(char-upcase code) → Int` — ASCII uppercase (LANG47).
pub fn char_upcase(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 { return Err(arity_error("char-upcase", 1, args.len())); }
    let cp = as_codepoint("char-upcase", args[0])?;
    let up = if is_lower(cp) { cp - 32 } else { cp };
    Ok(LispyValue::int(up as i64))
}

/// `(char-downcase code) → Int` — ASCII lowercase (LANG47).
pub fn char_downcase(args: &[LispyValue]) -> Result<LispyValue, RuntimeError> {
    if args.len() != 1 { return Err(arity_error("char-downcase", 1, args.len())); }
    let cp = as_codepoint("char-downcase", args[0])?;
    let down = if is_upper(cp) { cp + 32 } else { cp };
    Ok(LispyValue::int(down as i64))
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

    // ── _move / make_nil ────────────────────────────────────────────

    #[test]
    fn move_is_identity() {
        // _move preserves the exact bit pattern of its argument —
        // including booleans, which would otherwise coerce to int
        // through naive `add x 0` lowering.
        assert_eq!(move_(&[i(7)]).unwrap(), i(7));
        assert_eq!(move_(&[LispyValue::TRUE]).unwrap(), LispyValue::TRUE);
        assert_eq!(move_(&[LispyValue::FALSE]).unwrap(), LispyValue::FALSE);
        assert_eq!(move_(&[LispyValue::NIL]).unwrap(), LispyValue::NIL);
    }

    #[test]
    fn move_rejects_wrong_arity() {
        assert!(move_(&[]).is_err());
        assert!(move_(&[i(1), i(2)]).is_err());
    }

    #[test]
    fn make_nil_returns_nil() {
        assert_eq!(make_nil(&[]).unwrap(), LispyValue::NIL);
    }

    #[test]
    fn make_nil_rejects_args() {
        assert!(make_nil(&[i(1)]).is_err());
    }

    // ── String builtins (LANG47) ─────────────────────────────────────

    /// Helper: allocate a LispyValue string from a Rust str.
    fn s(text: &str) -> LispyValue {
        heap::alloc_string(text.as_bytes())
    }

    #[test]
    fn string_p_true_for_heap_string() {
        assert_eq!(string_p(&[s("hello")]).unwrap(), LispyValue::TRUE);
    }

    #[test]
    fn string_p_false_for_non_string() {
        assert_eq!(string_p(&[i(0)]).unwrap(), LispyValue::FALSE);
        assert_eq!(string_p(&[LispyValue::NIL]).unwrap(), LispyValue::FALSE);
        assert_eq!(string_p(&[LispyValue::symbol(intern("x"))]).unwrap(), LispyValue::FALSE);
    }

    #[test]
    fn string_length_ascii() {
        assert_eq!(string_length(&[s("hello")]).unwrap(), i(5));
        assert_eq!(string_length(&[s("")]).unwrap(), i(0));
        assert_eq!(string_length(&[s("a")]).unwrap(), i(1));
    }

    #[test]
    fn string_length_multibyte() {
        // "café" = 4 code points, 5 bytes.
        assert_eq!(string_length(&[s("café")]).unwrap(), i(4));
    }

    #[test]
    fn string_ref_returns_codepoint() {
        // 'h' = 104, 'e' = 101
        assert_eq!(string_ref(&[s("hello"), i(0)]).unwrap(), i(104));
        assert_eq!(string_ref(&[s("hello"), i(1)]).unwrap(), i(101));
    }

    #[test]
    fn string_ref_last_char() {
        // 'o' = 111
        assert_eq!(string_ref(&[s("hello"), i(4)]).unwrap(), i(111));
    }

    #[test]
    fn string_ref_out_of_bounds_errors() {
        assert!(string_ref(&[s("hi"), i(2)]).is_err());
        assert!(string_ref(&[s("hi"), i(-1)]).is_err());
    }

    #[test]
    fn substring_basic() {
        assert_eq!(
            unsafe { heap::string_bytes(substring(&[s("hello"), i(1), i(4)]).unwrap()).unwrap() },
            b"ell"
        );
    }

    #[test]
    fn substring_empty_range() {
        let v = substring(&[s("hello"), i(2), i(2)]).unwrap();
        assert_eq!(
            unsafe { heap::string_bytes(v).unwrap() },
            b""
        );
    }

    #[test]
    fn substring_invalid_range_errors() {
        // end < start is an error
        assert!(substring(&[s("hello"), i(3), i(1)]).is_err());
        // out of bounds
        assert!(substring(&[s("hello"), i(0), i(10)]).is_err());
    }

    #[test]
    fn string_append_concatenates() {
        let v = string_append(&[s("foo"), s("bar")]).unwrap();
        assert_eq!(
            unsafe { heap::string_bytes(v).unwrap() },
            b"foobar"
        );
    }

    #[test]
    fn string_append_empty() {
        let v = string_append(&[s(""), s("abc")]).unwrap();
        assert_eq!(unsafe { heap::string_bytes(v).unwrap() }, b"abc");
        let v2 = string_append(&[s("abc"), s("")]).unwrap();
        assert_eq!(unsafe { heap::string_bytes(v2).unwrap() }, b"abc");
    }

    #[test]
    fn make_string_builds_repeated_char() {
        let v = make_string(&[i(3), i(b'x' as i64)]).unwrap();
        assert_eq!(unsafe { heap::string_bytes(v).unwrap() }, b"xxx");
    }

    #[test]
    fn make_string_zero_length() {
        let v = make_string(&[i(0), i(b'a' as i64)]).unwrap();
        assert_eq!(unsafe { heap::string_bytes(v).unwrap() }.len(), 0);
    }

    #[test]
    fn string_eq_p_equal() {
        assert_eq!(string_eq_p(&[s("abc"), s("abc")]).unwrap(), LispyValue::TRUE);
    }

    #[test]
    fn string_eq_p_not_equal() {
        assert_eq!(string_eq_p(&[s("abc"), s("xyz")]).unwrap(), LispyValue::FALSE);
    }

    #[test]
    fn string_lt_p_ordering() {
        assert_eq!(string_lt_p(&[s("abc"), s("abd")]).unwrap(), LispyValue::TRUE);
        assert_eq!(string_lt_p(&[s("abd"), s("abc")]).unwrap(), LispyValue::FALSE);
        assert_eq!(string_lt_p(&[s("abc"), s("abc")]).unwrap(), LispyValue::FALSE);
    }

    #[test]
    fn string_gt_p_ordering() {
        assert_eq!(string_gt_p(&[s("abd"), s("abc")]).unwrap(), LispyValue::TRUE);
        assert_eq!(string_gt_p(&[s("abc"), s("abd")]).unwrap(), LispyValue::FALSE);
    }

    #[test]
    fn number_to_string_positive() {
        let v = number_to_string(&[i(42)]).unwrap();
        assert_eq!(unsafe { heap::string_bytes(v).unwrap() }, b"42");
    }

    #[test]
    fn number_to_string_negative() {
        let v = number_to_string(&[i(-7)]).unwrap();
        assert_eq!(unsafe { heap::string_bytes(v).unwrap() }, b"-7");
    }

    #[test]
    fn string_to_number_valid() {
        assert_eq!(string_to_number(&[s("42")]).unwrap(), i(42));
        assert_eq!(string_to_number(&[s("-7")]).unwrap(), i(-7));
    }

    #[test]
    fn string_to_number_invalid_returns_false() {
        assert_eq!(string_to_number(&[s("abc")]).unwrap(), LispyValue::FALSE);
        assert_eq!(string_to_number(&[s("")]).unwrap(), LispyValue::FALSE);
    }

    #[test]
    fn string_to_symbol_and_back() {
        let v = string_to_symbol(&[s("my-sym")]).unwrap();
        assert!(v.is_symbol());
        let back = symbol_to_string(&[v]).unwrap();
        assert_eq!(
            unsafe { heap::string_bytes(back).unwrap() },
            b"my-sym"
        );
    }

    #[test]
    fn char_alphabetic_p_ascii() {
        // 'a' = 97, 'Z' = 90, '5' = 53
        assert_eq!(char_alphabetic_p(&[i(97)]).unwrap(), LispyValue::TRUE);  // 'a'
        assert_eq!(char_alphabetic_p(&[i(90)]).unwrap(), LispyValue::TRUE);  // 'Z'
        assert_eq!(char_alphabetic_p(&[i(53)]).unwrap(), LispyValue::FALSE); // '5'
        assert_eq!(char_alphabetic_p(&[i(32)]).unwrap(), LispyValue::FALSE); // ' '
    }

    #[test]
    fn char_numeric_p_ascii() {
        assert_eq!(char_numeric_p(&[i(b'0' as i64)]).unwrap(), LispyValue::TRUE);
        assert_eq!(char_numeric_p(&[i(b'9' as i64)]).unwrap(), LispyValue::TRUE);
        assert_eq!(char_numeric_p(&[i(b'a' as i64)]).unwrap(), LispyValue::FALSE);
    }

    #[test]
    fn char_whitespace_p_various() {
        assert_eq!(char_whitespace_p(&[i(32)]).unwrap(),  LispyValue::TRUE);  // space
        assert_eq!(char_whitespace_p(&[i(10)]).unwrap(),  LispyValue::TRUE);  // newline
        assert_eq!(char_whitespace_p(&[i(9)]).unwrap(),   LispyValue::TRUE);  // tab
        assert_eq!(char_whitespace_p(&[i(b'a' as i64)]).unwrap(), LispyValue::FALSE);
    }

    #[test]
    fn char_upper_lower_case() {
        assert_eq!(char_upper_case_p(&[i(b'A' as i64)]).unwrap(), LispyValue::TRUE);
        assert_eq!(char_upper_case_p(&[i(b'a' as i64)]).unwrap(), LispyValue::FALSE);
        assert_eq!(char_lower_case_p(&[i(b'a' as i64)]).unwrap(), LispyValue::TRUE);
        assert_eq!(char_lower_case_p(&[i(b'A' as i64)]).unwrap(), LispyValue::FALSE);
    }

    #[test]
    fn char_upcase_downcase_ascii() {
        // 'a' (97) → 'A' (65);  'A' (65) → 'A' (65)
        assert_eq!(char_upcase(&[i(97)]).unwrap(), i(65));
        assert_eq!(char_upcase(&[i(65)]).unwrap(), i(65));
        // 'A' (65) → 'a' (97);  'a' (97) → 'a' (97)
        assert_eq!(char_downcase(&[i(65)]).unwrap(), i(97));
        assert_eq!(char_downcase(&[i(97)]).unwrap(), i(97));
    }

    #[test]
    fn char_to_integer_is_identity() {
        assert_eq!(char_to_integer(&[i(65)]).unwrap(), i(65));
    }

    #[test]
    fn integer_to_char_is_identity() {
        assert_eq!(integer_to_char(&[i(65)]).unwrap(), i(65));
    }

    // ── Extended comparisons (LANG52) ────────────────────────────────

    #[test]
    fn le_basic() {
        assert_eq!(le(&[i(1), i(2)]).unwrap(), LispyValue::TRUE);   // 1 ≤ 2
        assert_eq!(le(&[i(2), i(2)]).unwrap(), LispyValue::TRUE);   // 2 ≤ 2
        assert_eq!(le(&[i(3), i(2)]).unwrap(), LispyValue::FALSE);  // 3 ≤ 2 false
    }

    #[test]
    fn ge_basic() {
        assert_eq!(ge(&[i(2), i(1)]).unwrap(), LispyValue::TRUE);   // 2 ≥ 1
        assert_eq!(ge(&[i(2), i(2)]).unwrap(), LispyValue::TRUE);   // 2 ≥ 2
        assert_eq!(ge(&[i(1), i(2)]).unwrap(), LispyValue::FALSE);  // 1 ≥ 2 false
    }

    #[test]
    fn le_ge_reject_wrong_arity() {
        assert!(le(&[i(1)]).is_err());
        assert!(ge(&[i(1), i(2), i(3)]).is_err());
    }

    // ── Extended arithmetic (LANG52) ─────────────────────────────────

    #[test]
    fn quotient_basic() {
        // Same as truncating division.
        assert_eq!(quotient(&[i(13), i(4)]).unwrap(), i(3));
        assert_eq!(quotient(&[i(-13), i(4)]).unwrap(), i(-3));
        assert_eq!(quotient(&[i(13), i(-4)]).unwrap(), i(-3));
        assert_eq!(quotient(&[i(-13), i(-4)]).unwrap(), i(3));
    }

    #[test]
    fn quotient_by_zero_errors() {
        assert!(quotient(&[i(7), i(0)]).is_err());
    }

    #[test]
    fn remainder_basic() {
        // Sign matches dividend.
        assert_eq!(remainder(&[i(13), i(4)]).unwrap(), i(1));
        assert_eq!(remainder(&[i(-13), i(4)]).unwrap(), i(-1));
        assert_eq!(remainder(&[i(13), i(-4)]).unwrap(), i(1));
        assert_eq!(remainder(&[i(-13), i(-4)]).unwrap(), i(-1));
    }

    #[test]
    fn remainder_by_zero_errors() {
        assert!(remainder(&[i(5), i(0)]).is_err());
    }

    #[test]
    fn modulo_basic() {
        // Sign matches divisor (Scheme rule).
        assert_eq!(modulo(&[i(13), i(4)]).unwrap(), i(1));
        assert_eq!(modulo(&[i(-13), i(4)]).unwrap(), i(3));
        assert_eq!(modulo(&[i(13), i(-4)]).unwrap(), i(-3));
        assert_eq!(modulo(&[i(-13), i(-4)]).unwrap(), i(-1));
    }

    #[test]
    fn modulo_by_zero_errors() {
        assert!(modulo(&[i(7), i(0)]).is_err());
    }

    // ── Logic (LANG52) ────────────────────────────────────────────────

    #[test]
    fn boolean_p_true_for_bools() {
        assert_eq!(boolean_p(&[LispyValue::TRUE]).unwrap(), LispyValue::TRUE);
        assert_eq!(boolean_p(&[LispyValue::FALSE]).unwrap(), LispyValue::TRUE);
        assert_eq!(boolean_p(&[LispyValue::NIL]).unwrap(), LispyValue::FALSE);
        assert_eq!(boolean_p(&[i(0)]).unwrap(), LispyValue::FALSE);
    }

    #[test]
    fn not_returns_logical_inverse() {
        // Falsy inputs → #t
        assert_eq!(not(&[LispyValue::FALSE]).unwrap(), LispyValue::TRUE);
        assert_eq!(not(&[LispyValue::NIL]).unwrap(), LispyValue::TRUE);
        // Truthy inputs → #f  (0 is truthy in Scheme!)
        assert_eq!(not(&[LispyValue::TRUE]).unwrap(), LispyValue::FALSE);
        assert_eq!(not(&[i(0)]).unwrap(), LispyValue::FALSE);
        assert_eq!(not(&[i(42)]).unwrap(), LispyValue::FALSE);
    }

    #[test]
    fn not_rejects_wrong_arity() {
        assert!(not(&[]).is_err());
        assert!(not(&[LispyValue::TRUE, LispyValue::FALSE]).is_err());
    }

    #[test]
    fn equal_p_integers_by_value() {
        assert_eq!(equal_p(&[i(7), i(7)]).unwrap(), LispyValue::TRUE);
        assert_eq!(equal_p(&[i(7), i(8)]).unwrap(), LispyValue::FALSE);
    }

    #[test]
    fn equal_p_strings_by_content() {
        let s = |t: &str| heap::alloc_string(t.as_bytes());
        assert_eq!(equal_p(&[s("abc"), s("abc")]).unwrap(), LispyValue::TRUE);
        assert_eq!(equal_p(&[s("abc"), s("xyz")]).unwrap(), LispyValue::FALSE);
    }

    #[test]
    fn equal_p_lists_structurally() {
        // Two distinct allocations of (1 . 2) are equal?.
        let a = heap::alloc_cons(i(1), i(2));
        let b = heap::alloc_cons(i(1), i(2));
        assert_eq!(equal_p(&[a, b]).unwrap(), LispyValue::TRUE);
        // (1 2) vs (1 3) — differ in second element.
        let xs = heap::alloc_cons(i(1), heap::alloc_cons(i(2), LispyValue::NIL));
        let ys = heap::alloc_cons(i(1), heap::alloc_cons(i(3), LispyValue::NIL));
        assert_eq!(equal_p(&[xs, ys]).unwrap(), LispyValue::FALSE);
    }

    #[test]
    fn equal_p_symbols_by_identity() {
        let id_a = intern("alpha");
        let id_b = intern("beta");
        let sa = LispyValue::symbol(id_a);
        let sb = LispyValue::symbol(id_b);
        assert_eq!(equal_p(&[sa, sa]).unwrap(), LispyValue::TRUE);
        assert_eq!(equal_p(&[sa, sb]).unwrap(), LispyValue::FALSE);
    }

    // ── List operations (LANG52) ──────────────────────────────────────

    #[test]
    fn list_builds_proper_list() {
        // (list 1 2 3) → (1 2 3)
        let v = list(&[i(1), i(2), i(3)]).unwrap();
        assert_eq!(unsafe { heap::car(v) }.unwrap(), i(1));
        let tl = unsafe { heap::cdr(v) }.unwrap();
        assert_eq!(unsafe { heap::car(tl) }.unwrap(), i(2));
        let ttl = unsafe { heap::cdr(tl) }.unwrap();
        assert_eq!(unsafe { heap::car(ttl) }.unwrap(), i(3));
        assert!(unsafe { heap::cdr(ttl) }.unwrap().is_nil());
    }

    #[test]
    fn list_empty_is_nil() {
        assert_eq!(list(&[]).unwrap(), LispyValue::NIL);
    }

    #[test]
    fn list_p_proper_list_is_true() {
        let lst = list(&[i(1), i(2), i(3)]).unwrap();
        assert_eq!(list_p(&[lst]).unwrap(), LispyValue::TRUE);
        assert_eq!(list_p(&[LispyValue::NIL]).unwrap(), LispyValue::TRUE);
    }

    #[test]
    fn list_p_dotted_pair_is_false() {
        let pair = heap::alloc_cons(i(1), i(2)); // (1 . 2) — cdr is not nil
        assert_eq!(list_p(&[pair]).unwrap(), LispyValue::FALSE);
        assert_eq!(list_p(&[i(42)]).unwrap(), LispyValue::FALSE);
    }

    #[test]
    fn length_of_proper_list() {
        let lst = list(&[i(10), i(20), i(30)]).unwrap();
        assert_eq!(length(&[lst]).unwrap(), i(3));
        assert_eq!(length(&[LispyValue::NIL]).unwrap(), i(0));
    }

    #[test]
    fn length_of_non_list_errors() {
        let dotted = heap::alloc_cons(i(1), i(2));
        assert!(length(&[dotted]).is_err());
    }

    #[test]
    fn append_two_lists() {
        let a = list(&[i(1), i(2)]).unwrap();
        let b = list(&[i(3), i(4)]).unwrap();
        let result = append(&[a, b]).unwrap();
        assert_eq!(length(&[result]).unwrap(), i(4));
        assert_eq!(list_ref(&[result, i(0)]).unwrap(), i(1));
        assert_eq!(list_ref(&[result, i(3)]).unwrap(), i(4));
    }

    #[test]
    fn append_empty_first() {
        let b = list(&[i(1), i(2)]).unwrap();
        let result = append(&[LispyValue::NIL, b]).unwrap();
        assert_eq!(length(&[result]).unwrap(), i(2));
    }

    #[test]
    fn reverse_reverses() {
        let lst = list(&[i(1), i(2), i(3)]).unwrap();
        let rev = reverse(&[lst]).unwrap();
        assert_eq!(list_ref(&[rev, i(0)]).unwrap(), i(3));
        assert_eq!(list_ref(&[rev, i(1)]).unwrap(), i(2));
        assert_eq!(list_ref(&[rev, i(2)]).unwrap(), i(1));
    }

    #[test]
    fn reverse_empty_is_nil() {
        assert_eq!(reverse(&[LispyValue::NIL]).unwrap(), LispyValue::NIL);
    }

    #[test]
    fn list_ref_indexes_correctly() {
        let lst = list(&[i(10), i(20), i(30)]).unwrap();
        assert_eq!(list_ref(&[lst, i(0)]).unwrap(), i(10));
        assert_eq!(list_ref(&[lst, i(1)]).unwrap(), i(20));
        assert_eq!(list_ref(&[lst, i(2)]).unwrap(), i(30));
    }

    #[test]
    fn list_ref_out_of_bounds_errors() {
        let lst = list(&[i(1)]).unwrap();
        assert!(list_ref(&[lst, i(1)]).is_err());
        assert!(list_ref(&[lst, i(-1)]).is_err());
    }

    #[test]
    fn assoc_finds_key() {
        // Build ((1 . a) (2 . b) (3 . c))
        let sym_a = LispyValue::symbol(intern("a"));
        let sym_b = LispyValue::symbol(intern("b"));
        let sym_c = LispyValue::symbol(intern("c"));
        let alist = list(&[
            heap::alloc_cons(i(1), sym_a),
            heap::alloc_cons(i(2), sym_b),
            heap::alloc_cons(i(3), sym_c),
        ]).unwrap();
        let found = assoc(&[i(2), alist]).unwrap();
        assert!(unsafe { heap::is_cons(found) });
        assert_eq!(unsafe { heap::car(found) }.unwrap(), i(2));
        assert_eq!(unsafe { heap::cdr(found) }.unwrap(), sym_b);
    }

    #[test]
    fn assoc_not_found_returns_false() {
        let pair = heap::alloc_cons(i(1), LispyValue::symbol(intern("x")));
        let alist = list(&[pair]).unwrap();
        assert_eq!(assoc(&[i(99), alist]).unwrap(), LispyValue::FALSE);
    }

    #[test]
    fn assoc_empty_alist_returns_false() {
        assert_eq!(assoc(&[i(1), LispyValue::NIL]).unwrap(), LispyValue::FALSE);
    }

    // ── Symbol operations (LANG52) ────────────────────────────────────

    #[test]
    fn symbol_append_combines_names() {
        let foo = LispyValue::symbol(intern("foo"));
        let bar = LispyValue::symbol(intern("bar"));
        let result = symbol_append(&[foo, bar]).unwrap();
        assert!(result.is_symbol());
        let id = result.as_symbol().unwrap();
        let name = crate::intern::name_of(id).unwrap();
        assert_eq!(name, "foobar");
    }

    #[test]
    fn symbol_append_rejects_non_symbol() {
        let foo = LispyValue::symbol(intern("foo"));
        assert!(symbol_append(&[foo, i(1)]).is_err());
        assert!(symbol_append(&[i(1), foo]).is_err());
    }

    #[test]
    fn symbol_append_rejects_wrong_arity() {
        let foo = LispyValue::symbol(intern("foo"));
        assert!(symbol_append(&[foo]).is_err());
        assert!(symbol_append(&[]).is_err());
    }
}
