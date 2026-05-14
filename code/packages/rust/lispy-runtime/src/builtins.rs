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
}
