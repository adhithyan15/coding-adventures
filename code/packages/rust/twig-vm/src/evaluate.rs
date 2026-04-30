//! # 1-instruction evaluator: prove the substrate composes.
//!
//! This module is **not** a real interpreter — `vm-core` (PR 4)
//! is.  It's a focused integration test that takes a single
//! `call_builtin` IIR instruction, resolves the builtin through
//! [`LispyBinding`](lispy_runtime::LispyBinding), materialises
//! its argument operands into [`LispyValue`]s, and dispatches.
//! The output goes back as a `LispyValue`.
//!
//! What we prove with this:
//!
//! 1. The IIR a Twig program compiles to is **structurally
//!    consumable** by lispy-runtime's binding (the `srcs[0]` is
//!    a `Var` carrying a known builtin name; remaining `srcs`
//!    are operands convertible via [`crate::operand_to_value`]).
//! 2. The binding's `resolve_builtin` returns a fn pointer for
//!    every name the Twig frontend emits.
//! 3. Resolution + dispatch round-trips an arithmetic example
//!    end-to-end without ever touching vm-core.
//!
//! When PR 4 wires vm-core against `LangBinding`, the dispatch
//! loop will do this same shape on every `call_builtin` opcode —
//! the function in this file is a single iteration of that loop,
//! tested in isolation.

use interpreter_ir::{IIRInstr, Operand};
use lang_runtime_core::RuntimeError;
use lispy_runtime::{LispyBinding, LispyValue};

use crate::operand::operand_to_value;

/// Errors specific to the PR 3 evaluator.  Wraps
/// [`RuntimeError`] with extra context for the cases the
/// evaluator itself handles (wrong opcode, missing builtin
/// name, unresolved builtin).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluateError {
    /// The instruction's opcode wasn't `call_builtin`.  PR 3's
    /// evaluator only handles that one opcode; the rest land in
    /// PR 4 once vm-core is wired.
    UnsupportedOpcode(String),
    /// The instruction has no `srcs[0]` to interpret as the
    /// builtin name.
    MissingBuiltinName,
    /// `srcs[0]` was an `Operand::Var(name)` but `name` doesn't
    /// resolve through [`lispy_runtime::LispyBinding::resolve_builtin`].
    UnknownBuiltin(String),
    /// `srcs[0]` was something other than `Operand::Var` —
    /// builtin names always travel as Var operands by Twig
    /// convention.
    BuiltinNameNotVar,
    /// An argument failed to convert to a `LispyValue`.
    OperandConversion(RuntimeError),
    /// The builtin itself raised an error.
    BuiltinFailed(RuntimeError),
}

impl std::fmt::Display for EvaluateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvaluateError::UnsupportedOpcode(op) => {
                write!(f, "evaluate: opcode {op:?} not supported in PR 3 (PR 4 wires the full dispatcher)")
            }
            EvaluateError::MissingBuiltinName => {
                write!(f, "evaluate: call_builtin instruction has no srcs[0]")
            }
            EvaluateError::UnknownBuiltin(name) => {
                write!(f, "evaluate: no builtin named {name:?} in LispyBinding")
            }
            EvaluateError::BuiltinNameNotVar => {
                write!(f, "evaluate: srcs[0] of call_builtin must be Operand::Var(name)")
            }
            EvaluateError::OperandConversion(e) => {
                write!(f, "evaluate: operand conversion failed: {e}")
            }
            EvaluateError::BuiltinFailed(e) => {
                write!(f, "evaluate: builtin raised: {e}")
            }
        }
    }
}

impl std::error::Error for EvaluateError {}

/// Evaluate a single `call_builtin` instruction against
/// [`LispyBinding`].
///
/// `frame_lookup` resolves `Operand::Var(name)` argument operands
/// to their current `LispyValue`.  In tests, this is a closure
/// over a `HashMap`; in the future interpreter (PR 4), it's a
/// vm-core `VMFrame::resolve` call.
///
/// # Errors
///
/// See [`EvaluateError`] for the variant table.
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
/// use interpreter_ir::{IIRInstr, Operand};
/// use lispy_runtime::LispyValue;
/// use twig_vm::{evaluate_call_builtin};
///
/// // Synthesise: call_builtin "+" 2 3
/// let instr = IIRInstr::new(
///     "call_builtin",
///     Some("v0".into()),
///     vec![
///         Operand::Var("+".into()),
///         Operand::Int(2),
///         Operand::Int(3),
///     ],
///     "any",
/// );
/// let frame: HashMap<String, LispyValue> = HashMap::new();
/// let result = evaluate_call_builtin(&instr, &|name| frame.get(name).copied()).unwrap();
/// assert_eq!(result.as_int(), Some(5));
/// ```
pub fn evaluate_call_builtin(
    instr: &IIRInstr,
    frame_lookup: &dyn Fn(&str) -> Option<LispyValue>,
) -> Result<LispyValue, EvaluateError> {
    if instr.op != "call_builtin" {
        return Err(EvaluateError::UnsupportedOpcode(instr.op.clone()));
    }

    // srcs[0] = builtin name (always a Var by Twig convention)
    let name_operand = instr.srcs.first().ok_or(EvaluateError::MissingBuiltinName)?;
    let name = match name_operand {
        Operand::Var(s) => s.as_str(),
        _ => return Err(EvaluateError::BuiltinNameNotVar),
    };

    // Resolve through the binding.
    let builtin = <LispyBinding as lang_runtime_core::LangBinding>::resolve_builtin(name)
        .ok_or_else(|| EvaluateError::UnknownBuiltin(name.to_string()))?;

    // Convert remaining srcs to LispyValues.
    let mut args: Vec<LispyValue> = Vec::with_capacity(instr.srcs.len().saturating_sub(1));
    for src in &instr.srcs[1..] {
        let v = operand_to_value(src, frame_lookup)
            .map_err(EvaluateError::OperandConversion)?;
        args.push(v);
    }

    // Dispatch.
    builtin(&args).map_err(EvaluateError::BuiltinFailed)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn no_frame() -> HashMap<String, LispyValue> {
        HashMap::new()
    }

    fn lookup(map: &HashMap<String, LispyValue>) -> impl Fn(&str) -> Option<LispyValue> + '_ {
        |name| map.get(name).copied()
    }

    /// Build a synthetic `call_builtin "<name>" args…` instruction.
    fn call_builtin(name: &str, args: Vec<Operand>) -> IIRInstr {
        let mut srcs = vec![Operand::Var(name.into())];
        srcs.extend(args);
        IIRInstr::new("call_builtin", Some("v0".into()), srcs, "any")
    }

    // ── Arithmetic ──────────────────────────────────────────────────

    #[test]
    fn add_two_constants() {
        let instr = call_builtin("+", vec![Operand::Int(2), Operand::Int(3)]);
        let frame = no_frame();
        let r = evaluate_call_builtin(&instr, &lookup(&frame)).unwrap();
        assert_eq!(r.as_int(), Some(5));
    }

    #[test]
    fn nullary_plus_is_zero() {
        let instr = call_builtin("+", vec![]);
        let frame = no_frame();
        let r = evaluate_call_builtin(&instr, &lookup(&frame)).unwrap();
        assert_eq!(r.as_int(), Some(0));
    }

    #[test]
    fn variadic_plus() {
        let instr = call_builtin("+", vec![
            Operand::Int(1), Operand::Int(2), Operand::Int(3), Operand::Int(4),
        ]);
        let frame = no_frame();
        let r = evaluate_call_builtin(&instr, &lookup(&frame)).unwrap();
        assert_eq!(r.as_int(), Some(10));
    }

    #[test]
    fn subtraction_unary_negates() {
        let instr = call_builtin("-", vec![Operand::Int(7)]);
        let frame = no_frame();
        let r = evaluate_call_builtin(&instr, &lookup(&frame)).unwrap();
        assert_eq!(r.as_int(), Some(-7));
    }

    #[test]
    fn multiplication() {
        let instr = call_builtin("*", vec![Operand::Int(6), Operand::Int(7)]);
        let frame = no_frame();
        let r = evaluate_call_builtin(&instr, &lookup(&frame)).unwrap();
        assert_eq!(r.as_int(), Some(42));
    }

    #[test]
    fn division_by_zero_surfaces_as_builtin_error() {
        let instr = call_builtin("/", vec![Operand::Int(7), Operand::Int(0)]);
        let frame = no_frame();
        let err = evaluate_call_builtin(&instr, &lookup(&frame)).unwrap_err();
        match err {
            EvaluateError::BuiltinFailed(RuntimeError::TypeError(s)) => {
                assert!(s.contains("zero"));
            }
            other => panic!("expected BuiltinFailed(TypeError(\"...zero...\")), got {other:?}"),
        }
    }

    #[test]
    fn comparison_returns_bool() {
        let instr = call_builtin("<", vec![Operand::Int(1), Operand::Int(2)]);
        let frame = no_frame();
        let r = evaluate_call_builtin(&instr, &lookup(&frame)).unwrap();
        assert_eq!(r, LispyValue::TRUE);

        let instr2 = call_builtin("<", vec![Operand::Int(5), Operand::Int(5)]);
        let r2 = evaluate_call_builtin(&instr2, &lookup(&frame)).unwrap();
        assert_eq!(r2, LispyValue::FALSE);
    }

    // ── Cons / car / cdr ────────────────────────────────────────────

    #[test]
    fn cons_car_cdr_round_trip() {
        // cons 1 2 → pair; car pair → 1; cdr pair → 2.
        let cons = call_builtin("cons", vec![Operand::Int(1), Operand::Int(2)]);
        let frame = no_frame();
        let pair = evaluate_call_builtin(&cons, &lookup(&frame)).unwrap();
        assert!(pair.is_heap());

        // To call car/cdr we need the pair as an argument; since
        // the pair came from this evaluator it's a "live"
        // LispyValue.  We pass it by stashing in the frame.
        let mut frame2: HashMap<String, LispyValue> = HashMap::new();
        frame2.insert("p".into(), pair);
        let car_instr = call_builtin("car", vec![Operand::Var("p".into())]);
        let cdr_instr = call_builtin("cdr", vec![Operand::Var("p".into())]);
        assert_eq!(
            evaluate_call_builtin(&car_instr, &lookup(&frame2)).unwrap().as_int(),
            Some(1),
        );
        assert_eq!(
            evaluate_call_builtin(&cdr_instr, &lookup(&frame2)).unwrap().as_int(),
            Some(2),
        );
    }

    #[test]
    fn null_p_only_true_for_nil_value_in_frame() {
        // Test the null? predicate via a frame lookup since the
        // IR compiler routes `nil` through `make_nil` builtin.
        let mut frame: HashMap<String, LispyValue> = HashMap::new();
        frame.insert("n".into(), LispyValue::NIL);
        frame.insert("z".into(), LispyValue::int(0));
        let null_n = call_builtin("null?", vec![Operand::Var("n".into())]);
        let null_z = call_builtin("null?", vec![Operand::Var("z".into())]);
        assert_eq!(evaluate_call_builtin(&null_n, &lookup(&frame)).unwrap(), LispyValue::TRUE);
        assert_eq!(evaluate_call_builtin(&null_z, &lookup(&frame)).unwrap(), LispyValue::FALSE);
    }

    // ── Error paths ─────────────────────────────────────────────────

    #[test]
    fn non_call_builtin_opcode_rejected() {
        let instr = IIRInstr::new("ret", None, vec![], "any");
        let err = evaluate_call_builtin(&instr, &|_| None).unwrap_err();
        assert!(matches!(err, EvaluateError::UnsupportedOpcode(s) if s == "ret"));
    }

    #[test]
    fn missing_srcs_rejected() {
        let instr = IIRInstr::new("call_builtin", None, vec![], "any");
        let err = evaluate_call_builtin(&instr, &|_| None).unwrap_err();
        assert_eq!(err, EvaluateError::MissingBuiltinName);
    }

    #[test]
    fn non_var_first_src_rejected() {
        let instr = IIRInstr::new(
            "call_builtin",
            None,
            vec![Operand::Int(0)], // wrong shape — name should be Var
            "any",
        );
        let err = evaluate_call_builtin(&instr, &|_| None).unwrap_err();
        assert_eq!(err, EvaluateError::BuiltinNameNotVar);
    }

    #[test]
    fn unknown_builtin_rejected() {
        let instr = call_builtin("does_not_exist", vec![]);
        let err = evaluate_call_builtin(&instr, &|_| None).unwrap_err();
        assert!(matches!(err, EvaluateError::UnknownBuiltin(s) if s == "does_not_exist"));
    }

    #[test]
    fn operand_conversion_error_propagates() {
        // Var operand referencing a name not in the frame.
        let instr = call_builtin("+", vec![Operand::Var("missing".into())]);
        let err = evaluate_call_builtin(&instr, &|_| None).unwrap_err();
        assert!(matches!(err, EvaluateError::OperandConversion(_)));
    }
}
