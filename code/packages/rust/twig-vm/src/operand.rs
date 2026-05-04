//! # IIR `Operand` → `LispyValue` conversion.
//!
//! This is the **per-language seam** between the language-agnostic
//! IIR enum and the Lispy runtime's tagged-i64 value
//! representation.  When the interpreter (PR 4+) executes a
//! `call_builtin` instruction, it has to materialise each
//! [`interpreter_ir::Operand`] argument into a [`LispyValue`]
//! before handing the slice to the resolved
//! [`BuiltinFn<LispyBinding>`](lang_runtime_core::BuiltinFn).
//! That's what this module does.
//!
//! ## Mapping table
//!
//! | `Operand` variant | `LispyValue` |
//! |-------------------|--------------|
//! | `Operand::Int(n)` | `LispyValue::int(n)` |
//! | `Operand::Bool(b)` | `LispyValue::bool(b)` |
//! | `Operand::Var("nil")` | `LispyValue::NIL` (special-cased name) |
//! | `Operand::Var(name)` | resolved through the supplied `frame_lookup` callback |
//! | `Operand::Float(f)` | currently unsupported (PR 2 has no flonums) — returns an error |
//!
//! ## Why pass `frame_lookup` as a callback rather than a field?
//!
//! In the future interpreter (PR 4), the register file is owned
//! by `vm-core::VMFrame`.  Threading it through this conversion
//! function as a closure keeps `operand.rs` decoupled from any
//! specific VMFrame layout — when vm-core wires up against
//! `LangBinding`, it just passes its own resolver.
//!
//! For PR 3's tests, callers pass a simple `HashMap<String,
//! LispyValue>`-backed closure.  Same shape, same code path.

use lang_runtime_core::RuntimeError;

use lispy_runtime::LispyValue;

use interpreter_ir::Operand;

/// Convert an IIR `Operand` to a `LispyValue`, using `frame_lookup`
/// to resolve `Operand::Var(name)` references.
///
/// `frame_lookup` is invoked only for `Var` operands.  If it
/// returns `None` for a name, this function returns
/// `RuntimeError::Custom("undefined variable: …")` — matching
/// vm-core's convention.
///
/// # Special case: `nil`
///
/// The TW00 IR compiler emits `call_builtin "make_nil"` → produces
/// a fresh `nil` register; bare `nil` references go through the
/// register file like any other local.  But during builtin name
/// resolution the compiler embeds names like `"_move"`, `"+"`,
/// etc. as `Operand::Var(name)` for the first arg of
/// `call_builtin`.  These are never values — the dispatcher
/// peels them off before reaching this function.
///
/// ## Range and overflow
///
/// `Operand::Int(n)` accepts the full `i64` range, but
/// `LispyValue::int` is bounded to ±2⁶⁰ (lispy-runtime's tagged-int
/// range).  Out-of-range integers — which the parser cannot produce
/// today (the IIR's `int_literal` opcode goes through Twig's parser
/// which limits to i64 with a debug assert in lispy-runtime) — return
/// a `RuntimeError::TypeError` here rather than triggering the assert.
pub fn operand_to_value(
    operand: &Operand,
    frame_lookup: &dyn Fn(&str) -> Option<LispyValue>,
) -> Result<LispyValue, RuntimeError> {
    use lispy_runtime::{TAG_INT, TAG_HEAP, TAG_SYMBOL};
    let _ = TAG_INT; // touch re-exports so the import is intentional
    let _ = TAG_HEAP;
    let _ = TAG_SYMBOL;
    match operand {
        Operand::Int(n) => {
            // Bound check — lispy-runtime's tagged-int range is
            // ±2⁶⁰, narrower than i64.  Values outside that get
            // surfaced as a runtime error rather than the
            // `LispyValue::int` debug-assert.
            const MAX: i64 = (1 << 60) - 1;
            const MIN: i64 = -(1 << 60);
            if !(MIN..=MAX).contains(n) {
                return Err(RuntimeError::TypeError(format!(
                    "integer literal {n} outside Lispy's tagged-int range \
                     [-2^60, 2^60 - 1] — cannot represent without bignums"
                )));
            }
            Ok(LispyValue::int(*n))
        }
        Operand::Bool(b) => Ok(LispyValue::bool(*b)),
        Operand::Float(_f) => Err(RuntimeError::TypeError(
            "Lispy doesn't have flonums yet (PR 2 ships int + bool + nil + symbol)".into(),
        )),
        Operand::Var(name) => {
            // Special case: TW00's IR compiler doesn't emit
            // `Operand::Var("nil")` directly (it routes through
            // `make_nil`), so we don't need a fallthrough — but
            // we accept it for forward-compat with simpler test
            // helpers.
            if name == "nil" {
                return Ok(LispyValue::NIL);
            }
            frame_lookup(name).ok_or_else(|| {
                RuntimeError::Custom(format!("undefined variable: {name}"))
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Build a `frame_lookup` from a `HashMap`.
    fn lookup(map: &HashMap<String, LispyValue>) -> impl Fn(&str) -> Option<LispyValue> + '_ {
        |name| map.get(name).copied()
    }

    fn no_lookup(_: &str) -> Option<LispyValue> {
        None
    }

    #[test]
    fn int_operand_round_trips() {
        let v = operand_to_value(&Operand::Int(42), &no_lookup).unwrap();
        assert_eq!(v.as_int(), Some(42));
    }

    #[test]
    fn negative_int_operand_round_trips() {
        let v = operand_to_value(&Operand::Int(-7), &no_lookup).unwrap();
        assert_eq!(v.as_int(), Some(-7));
    }

    #[test]
    fn int_at_lispy_range_boundary_works() {
        let max = (1i64 << 60) - 1;
        let min = -(1i64 << 60);
        assert_eq!(operand_to_value(&Operand::Int(max), &no_lookup).unwrap().as_int(), Some(max));
        assert_eq!(operand_to_value(&Operand::Int(min), &no_lookup).unwrap().as_int(), Some(min));
    }

    #[test]
    fn int_above_lispy_range_returns_error() {
        let too_big = 1i64 << 60; // INT_MAX + 1
        let err = operand_to_value(&Operand::Int(too_big), &no_lookup).unwrap_err();
        assert!(matches!(err, RuntimeError::TypeError(s) if s.contains("tagged-int range")));
    }

    #[test]
    fn int_below_lispy_range_returns_error() {
        let too_small = i64::MIN; // far below -2^60
        let err = operand_to_value(&Operand::Int(too_small), &no_lookup).unwrap_err();
        assert!(matches!(err, RuntimeError::TypeError(s) if s.contains("tagged-int range")));
    }

    #[test]
    fn bool_operand_round_trips() {
        assert_eq!(operand_to_value(&Operand::Bool(true), &no_lookup).unwrap(), LispyValue::TRUE);
        assert_eq!(operand_to_value(&Operand::Bool(false), &no_lookup).unwrap(), LispyValue::FALSE);
    }

    #[test]
    fn float_operand_returns_unsupported_error() {
        let err = operand_to_value(&Operand::Float(1.5), &no_lookup).unwrap_err();
        assert!(matches!(err, RuntimeError::TypeError(s) if s.contains("flonum")));
    }

    #[test]
    fn var_resolves_through_lookup() {
        let mut map = HashMap::new();
        map.insert("x".to_string(), LispyValue::int(99));
        let v = operand_to_value(&Operand::Var("x".into()), &lookup(&map)).unwrap();
        assert_eq!(v, LispyValue::int(99));
    }

    #[test]
    fn missing_var_returns_undefined_error() {
        let err = operand_to_value(&Operand::Var("missing".into()), &no_lookup).unwrap_err();
        assert!(matches!(err, RuntimeError::Custom(s) if s.contains("undefined variable")));
    }

    #[test]
    fn nil_special_case_resolves_to_lispy_nil() {
        // Test helpers can pass `Operand::Var("nil")` directly
        // without populating the lookup map.
        let v = operand_to_value(&Operand::Var("nil".into()), &no_lookup).unwrap();
        assert_eq!(v, LispyValue::NIL);
    }
}
