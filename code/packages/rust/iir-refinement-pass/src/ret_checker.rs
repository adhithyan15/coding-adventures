//! Return-site refinement checker for the `iir-refinement-pass`.
//!
//! # Responsibility
//!
//! For each function that has a `return_refinement`, this module scans the
//! function's instruction list for every `ret` instruction and checks whether
//! the returned value provably satisfies the declared return annotation.
//!
//! The evidence-resolution and outcome-handling logic is identical to the
//! call-site checker ([`call_checker`](crate::call_checker)): we use the
//! function's [`ConstMap`] to turn constant-valued variable references into
//! concrete evidence, and we respect the current [`RefinementMode`] when
//! deciding whether `Unknown` outcomes become errors.
//!
//! # Example
//!
//! ```text
//! ; function annotated `-> (Int 0 256)`
//! const v = 500
//! ret   v         ; v is tracked in ConstMap → Evidence::Concrete(500) → PROVEN_UNSAFE
//! ```

use interpreter_ir::instr::Operand;
use interpreter_ir::function::IIRFunction;
use lang_refinement_checker::{Checker, Evidence, CheckOutcome};

use crate::const_prop::ConstMap;
use crate::{RefinementError, RefinementMode};

/// Check every `ret` instruction in `func` against `func.return_refinement`.
///
/// If the function carries no `return_refinement` the function returns
/// immediately with no errors.  Results are appended to `errors`.
pub fn check_returns(
    func: &IIRFunction,
    const_map: &ConstMap,
    mode: RefinementMode,
    errors: &mut Vec<RefinementError>,
) {
    // Fast path: no return annotation means nothing to check.
    let annotation = match &func.return_refinement {
        Some(ann) => ann,
        None => return,
    };

    let mut checker = Checker::new();

    for instr in &func.instructions {
        if instr.op != "ret" {
            continue;
        }

        // Resolve the returned value's evidence.
        let evidence = match instr.srcs.first() {
            Some(Operand::Int(v))   => Evidence::Concrete(*v as i128),
            Some(Operand::Var(name)) => {
                if let Some(&v) = const_map.get(name.as_str()) {
                    Evidence::Concrete(v)
                } else {
                    Evidence::Unconstrained
                }
            }
            _ => Evidence::Unconstrained,
        };

        let outcome = checker.check(annotation, &evidence);

        match outcome {
            CheckOutcome::ProvenSafe => {}
            CheckOutcome::ProvenUnsafe(cx) => {
                errors.push(RefinementError {
                    function: func.name.clone(),
                    site: "return site".to_string(),
                    counter_example: cx.value,
                    description: cx.description,
                });
            }
            CheckOutcome::Unknown(msg) => {
                if mode == RefinementMode::Strict {
                    errors.push(RefinementError {
                        function: func.name.clone(),
                        site: "return site (UNKNOWN in strict mode)".to_string(),
                        counter_example: 0,
                        description: msg,
                    });
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::function::IIRFunction;
    use interpreter_ir::instr::{IIRInstr, Operand};
    use lang_refined_types::{RefinedType, Kind};
    use lang_refined_types::Predicate;
    use std::collections::HashMap;

    /// `(Int 0 255)` — closed range [0, 255].
    fn byte_annotation() -> RefinedType {
        RefinedType::refined(
            Kind::Int,
            Predicate::Range {
                lo: Some(0),
                hi: Some(255),
                inclusive_hi: true,
            },
        )
    }

    fn func_returning_literal(literal: i64, annotation: RefinedType) -> IIRFunction {
        IIRFunction {
            name: "f".into(),
            return_refinement: Some(annotation),
            instructions: vec![
                IIRInstr::new("ret", None, vec![Operand::Int(literal)], "i64"),
            ],
            ..Default::default()
        }
    }

    #[test]
    fn return_literal_in_range_no_error() {
        // 42 ∈ [0, 255] → ProvenSafe → no error.
        let func = func_returning_literal(42, byte_annotation());
        let mut errors = Vec::new();
        check_returns(&func, &HashMap::new(), RefinementMode::Lenient, &mut errors);
        assert!(errors.is_empty());
    }

    #[test]
    fn return_literal_out_of_range_error() {
        // 300 ∉ [0, 255] → ProvenUnsafe → error.
        let func = func_returning_literal(300, byte_annotation());
        let mut errors = Vec::new();
        check_returns(&func, &HashMap::new(), RefinementMode::Lenient, &mut errors);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].counter_example, 300);
        assert_eq!(errors[0].site, "return site");
    }

    #[test]
    fn return_const_tracked_variable_violation() {
        // `const v = 999; ret v` with v tracked → caught.
        let ann = byte_annotation();
        let func = IIRFunction {
            name: "f".into(),
            return_refinement: Some(ann),
            instructions: vec![
                IIRInstr::new("const", Some("v".into()), vec![Operand::Int(999)], "i64"),
                IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i64"),
            ],
            ..Default::default()
        };
        let mut const_map = HashMap::new();
        const_map.insert("v".to_string(), 999i128);
        let mut errors = Vec::new();
        check_returns(&func, &const_map, RefinementMode::Lenient, &mut errors);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].counter_example, 999);
    }

    #[test]
    fn return_unconstrained_lenient_silent() {
        // Unknown variable → Unconstrained → UNKNOWN → silent in Lenient.
        let func = IIRFunction {
            name: "f".into(),
            return_refinement: Some(byte_annotation()),
            instructions: vec![
                IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "i64"),
            ],
            ..Default::default()
        };
        let mut errors = Vec::new();
        check_returns(&func, &HashMap::new(), RefinementMode::Lenient, &mut errors);
        assert!(errors.is_empty());
    }

    #[test]
    fn return_unconstrained_strict_error() {
        // Unknown variable → UNKNOWN → error in Strict.
        let func = IIRFunction {
            name: "f".into(),
            return_refinement: Some(byte_annotation()),
            instructions: vec![
                IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "i64"),
            ],
            ..Default::default()
        };
        let mut errors = Vec::new();
        check_returns(&func, &HashMap::new(), RefinementMode::Strict, &mut errors);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_return_annotation_skipped() {
        // Function with no return_refinement → fast path → no errors even for
        // an obviously bad literal.
        let func = IIRFunction {
            name: "f".into(),
            return_refinement: None,
            instructions: vec![
                IIRInstr::new("ret", None, vec![Operand::Int(99999)], "i64"),
            ],
            ..Default::default()
        };
        let mut errors = Vec::new();
        check_returns(&func, &HashMap::new(), RefinementMode::Strict, &mut errors);
        assert!(errors.is_empty(), "no annotation → nothing to check");
    }

    #[test]
    fn multiple_ret_sites_all_checked() {
        // Two `ret` instructions in the same function → both checked.
        let ann = byte_annotation();
        let func = IIRFunction {
            name: "f".into(),
            return_refinement: Some(ann),
            instructions: vec![
                IIRInstr::new("ret", None, vec![Operand::Int(42)],  "i64"),
                IIRInstr::new("ret", None, vec![Operand::Int(300)], "i64"),
            ],
            ..Default::default()
        };
        let mut errors = Vec::new();
        check_returns(&func, &HashMap::new(), RefinementMode::Lenient, &mut errors);
        // Only 300 violates the annotation.
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].counter_example, 300);
    }
}
