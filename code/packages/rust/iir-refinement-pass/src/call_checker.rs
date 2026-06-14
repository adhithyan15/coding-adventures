//! Call-site refinement checker for the `iir-refinement-pass`.
//!
//! # Responsibility
//!
//! For every `call` instruction in a function, this module:
//!
//! 1. Resolves the callee name from `srcs[0]` (a `Var` operand).
//! 2. Looks up the callee's [`IIRFunction`] in the module to find its
//!    `param_refinements`.
//! 3. For each argument `srcs[i+1]` that has a corresponding annotation,
//!    resolves evidence via the constant-propagation map and calls
//!    [`Checker::check`].
//! 4. Translates each [`CheckOutcome`] into zero or one [`RefinementError`]
//!    entries according to the current [`RefinementMode`].
//!
//! # Evidence resolution
//!
//! | Argument operand | Evidence |
//! |---|---|
//! | `Operand::Int(v)` | `Evidence::Concrete(v as i128)` |
//! | `Operand::Var(name)` where `name ∈ ConstMap` | `Evidence::Concrete(map[name])` |
//! | `Operand::Var(name)` where `name ∉ ConstMap` | `Evidence::Unconstrained` |
//! | `Operand::Bool(_)` / `Operand::Float(_)` / `Operand::Str(_)` | `Evidence::Unconstrained` |
//!
//! # Mode behaviour
//!
//! - **Lenient**: only `ProvenUnsafe` outcomes become errors.
//! - **Strict**: both `ProvenUnsafe` and `Unknown` outcomes become errors.

use interpreter_ir::instr::Operand;
use interpreter_ir::module::IIRModule;
use interpreter_ir::function::IIRFunction;
use lang_refinement_checker::{Checker, Evidence, CheckOutcome};

use crate::const_prop::ConstMap;
use crate::{RefinementError, RefinementMode};

/// Check every `call` instruction in `func` against the callee's
/// `param_refinements`.
///
/// Results are appended to `errors`.  The function is the owner of the caller
/// context (used in error messages), and `module` is needed to look up each
/// callee.
pub fn check_calls(
    func: &IIRFunction,
    module: &IIRModule,
    const_map: &ConstMap,
    mode: RefinementMode,
    errors: &mut Vec<RefinementError>,
) {
    let mut checker = Checker::new();

    for instr in &func.instructions {
        // Only `call` instructions carry argument evidence.
        if instr.op != "call" {
            continue;
        }

        // srcs[0] = callee name (Var).  srcs[1..] = arguments.
        let callee_name = match instr.srcs.first() {
            Some(Operand::Var(name)) => name.as_str(),
            _ => continue, // malformed call; skip gracefully
        };

        // Look up the callee function definition.
        let callee = match module.get_function(callee_name) {
            Some(f) => f,
            None => continue, // external function; no annotation to check
        };

        // Nothing to check if the callee has no refinement annotations.
        if callee.param_refinements.is_empty() {
            continue;
        }

        // Walk arguments (srcs[1..]) and check each annotated parameter.
        for (i, arg_operand) in instr.srcs[1..].iter().enumerate() {
            // Get the annotation for parameter i (if any).
            let annotation = match callee.param_refinements.get(i) {
                Some(Some(ann)) => ann,
                _ => continue, // no annotation for this parameter index
            };

            // Resolve compile-time evidence for this argument.
            let evidence = resolve_evidence(arg_operand, const_map);

            // Ask the solver.
            let outcome = checker.check(annotation, &evidence);

            match outcome {
                CheckOutcome::ProvenSafe => {
                    // No error — value provably satisfies the annotation.
                }
                CheckOutcome::ProvenUnsafe(cx) => {
                    errors.push(RefinementError {
                        function: func.name.clone(),
                        site: format!(
                            "call to `{}`, argument {}",
                            callee_name, i
                        ),
                        counter_example: cx.value,
                        description: cx.description,
                    });
                }
                CheckOutcome::Unknown(msg) => {
                    if mode == RefinementMode::Strict {
                        errors.push(RefinementError {
                            function: func.name.clone(),
                            site: format!(
                                "call to `{}`, argument {} (UNKNOWN in strict mode)",
                                callee_name, i
                            ),
                            counter_example: 0,
                            description: msg,
                        });
                    }
                    // In Lenient mode, UNKNOWN is silently ignored.
                }
            }
        }
    }
}

/// Resolve compile-time evidence from a single `call` argument operand.
///
/// - `Operand::Int(v)` → `Concrete(v as i128)` (literal in the call itself)
/// - `Operand::Var(name)` in `const_map` → `Concrete(map[name])`
/// - everything else → `Unconstrained`
fn resolve_evidence(operand: &Operand, const_map: &ConstMap) -> Evidence {
    match operand {
        Operand::Int(v) => Evidence::Concrete(*v as i128),
        Operand::Var(name) => {
            if let Some(&v) = const_map.get(name.as_str()) {
                Evidence::Concrete(v)
            } else {
                Evidence::Unconstrained
            }
        }
        _ => Evidence::Unconstrained,
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
    use interpreter_ir::module::IIRModule;
    use lang_refined_types::{RefinedType, Kind};
    use lang_refined_types::Predicate;
    use std::collections::HashMap;

    /// Build a module with two functions: caller and callee.
    ///
    /// `callee` has `param_refinements[0] = Some(annotation)`.
    /// `caller` has the provided instruction list.
    fn make_module(
        annotation: RefinedType,
        caller_instrs: Vec<IIRInstr>,
    ) -> IIRModule {
        // Callee: takes one parameter annotated with `annotation`.
        let callee = IIRFunction {
            name: "callee".into(),
            params: vec![("x".into(), "i64".into())],
            param_refinements: vec![Some(annotation)],
            instructions: vec![
                IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "i64"),
            ],
            ..Default::default()
        };

        // Caller: user-supplied instructions.
        let caller = IIRFunction {
            name: "caller".into(),
            instructions: caller_instrs,
            ..Default::default()
        };

        let mut module = IIRModule::new("test.twig", "twig");
        module.functions.push(callee);
        module.functions.push(caller);
        module
    }

    /// `(Int 0 128)` annotation: valid range [0, 128).
    fn range_0_128() -> RefinedType {
        RefinedType::refined(
            Kind::Int,
            Predicate::Range {
                lo: Some(0),
                hi: Some(127),
                inclusive_hi: true,
            },
        )
    }

    #[test]
    fn literal_in_range_no_error() {
        // Call callee with literal 42 — in [0, 128) — no error.
        let call_instr = IIRInstr::new(
            "call",
            Some("r".into()),
            vec![Operand::Var("callee".into()), Operand::Int(42)],
            "i64",
        );
        let module = make_module(range_0_128(), vec![call_instr]);
        let caller = module.get_function("caller").unwrap();
        let mut errors = Vec::new();
        check_calls(caller, &module, &HashMap::new(), RefinementMode::Lenient, &mut errors);
        assert!(errors.is_empty(), "no error expected for value 42 in [0,128)");
    }

    #[test]
    fn literal_out_of_range_error() {
        // Call callee with literal 200 — violates [0, 128) → error.
        let call_instr = IIRInstr::new(
            "call",
            Some("r".into()),
            vec![Operand::Var("callee".into()), Operand::Int(200)],
            "i64",
        );
        let module = make_module(range_0_128(), vec![call_instr]);
        let caller = module.get_function("caller").unwrap();
        let mut errors = Vec::new();
        check_calls(caller, &module, &HashMap::new(), RefinementMode::Lenient, &mut errors);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].counter_example, 200);
    }

    #[test]
    fn const_tracked_variable_violation() {
        // const arg0 = 500, then call callee arg0 → caught via ConstMap.
        let const_instr = IIRInstr::new(
            "const",
            Some("arg0".into()),
            vec![Operand::Int(500)],
            "i64",
        );
        let call_instr = IIRInstr::new(
            "call",
            Some("r".into()),
            vec![Operand::Var("callee".into()), Operand::Var("arg0".into())],
            "i64",
        );
        let module = make_module(range_0_128(), vec![const_instr.clone(), call_instr]);
        let caller = module.get_function("caller").unwrap();
        // Build const_map manually (as lib.rs would).
        let mut const_map = HashMap::new();
        const_map.insert("arg0".to_string(), 500i128);
        let mut errors = Vec::new();
        check_calls(caller, &module, &const_map, RefinementMode::Lenient, &mut errors);
        assert_eq!(errors.len(), 1, "500 violates [0,128)");
        assert_eq!(errors[0].counter_example, 500);
    }

    #[test]
    fn unconstrained_variable_lenient_silent() {
        // An unknown variable → Unconstrained → UNKNOWN outcome → silent in Lenient.
        let call_instr = IIRInstr::new(
            "call",
            Some("r".into()),
            vec![Operand::Var("callee".into()), Operand::Var("unknown_var".into())],
            "i64",
        );
        let module = make_module(range_0_128(), vec![call_instr]);
        let caller = module.get_function("caller").unwrap();
        let mut errors = Vec::new();
        check_calls(caller, &module, &HashMap::new(), RefinementMode::Lenient, &mut errors);
        assert!(errors.is_empty(), "unconstrained should be silent in Lenient mode");
    }

    #[test]
    fn unconstrained_variable_strict_error() {
        // Same unknown variable → UNKNOWN → error in Strict mode.
        let call_instr = IIRInstr::new(
            "call",
            Some("r".into()),
            vec![Operand::Var("callee".into()), Operand::Var("unknown_var".into())],
            "i64",
        );
        let module = make_module(range_0_128(), vec![call_instr]);
        let caller = module.get_function("caller").unwrap();
        let mut errors = Vec::new();
        check_calls(caller, &module, &HashMap::new(), RefinementMode::Strict, &mut errors);
        assert_eq!(errors.len(), 1, "UNKNOWN should be an error in Strict mode");
    }

    #[test]
    fn unannotated_callee_skipped() {
        // Callee with no param_refinements → always skipped.
        let unannotated_callee = IIRFunction {
            name: "unann".into(),
            params: vec![("x".into(), "i64".into())],
            param_refinements: vec![], // empty — no annotations
            ..Default::default()
        };
        let call_instr = IIRInstr::new(
            "call",
            Some("r".into()),
            vec![Operand::Var("unann".into()), Operand::Int(9999)],
            "i64",
        );
        let caller = IIRFunction {
            name: "caller".into(),
            instructions: vec![call_instr],
            ..Default::default()
        };
        let mut module = IIRModule::new("test.twig", "twig");
        module.functions.push(unannotated_callee);
        module.functions.push(caller);

        let caller_fn = module.get_function("caller").unwrap();
        let mut errors = Vec::new();
        check_calls(caller_fn, &module, &HashMap::new(), RefinementMode::Strict, &mut errors);
        assert!(errors.is_empty(), "unannotated callee must be skipped entirely");
    }

    #[test]
    fn multiple_violations_all_reported() {
        // Callee with two annotated params; both violated → two errors.
        let callee = IIRFunction {
            name: "two_params".into(),
            params: vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
            param_refinements: vec![Some(range_0_128()), Some(range_0_128())],
            ..Default::default()
        };
        // Both 200 and 300 violate [0, 128).
        let call_instr = IIRInstr::new(
            "call",
            Some("r".into()),
            vec![
                Operand::Var("two_params".into()),
                Operand::Int(200),
                Operand::Int(300),
            ],
            "i64",
        );
        let caller = IIRFunction {
            name: "caller".into(),
            instructions: vec![call_instr],
            ..Default::default()
        };
        let mut module = IIRModule::new("test.twig", "twig");
        module.functions.push(callee);
        module.functions.push(caller);

        let caller_fn = module.get_function("caller").unwrap();
        let mut errors = Vec::new();
        check_calls(caller_fn, &module, &HashMap::new(), RefinementMode::Lenient, &mut errors);
        assert_eq!(errors.len(), 2, "both violations should be reported");
    }
}
