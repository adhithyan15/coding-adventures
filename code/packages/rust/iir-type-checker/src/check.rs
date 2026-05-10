//! `check_module` — validation pass over an `IIRModule`.
//!
//! This pass **reads** the module but never modifies it.  It produces a
//! [`TypeCheckReport`] that describes:
//!
//! - How typed the module is (its [`TypingTier`]).
//! - Any type errors found (invalid type strings, mismatched operand types,
//!   ill-typed branch conditions).
//!
//! The pass is deliberately **lenient with `"any"` instructions** — they
//! produce neither errors nor warnings (they are simply untyped and counted
//! in the fraction denominator).  Only instructions whose `type_hint` is
//! not `"any"` and not a valid concrete type are flagged as `InvalidType`.
//!
//! # Algorithm
//!
//! For each function, for each instruction:
//!
//! 1. **Skip `"any"` instructions** — no validation, just count.
//! 2. **Skip control-flow ops** (`jmp`, `jmp_if_*`, `label`, `ret_void`)
//!    for the typed-fraction denominator: they carry no data type.
//! 3. **Validate type_hint**: must be `"any"`, in `CONCRETE_TYPES`, or a
//!    `ref<T>` form.
//! 4. **Validate binary-op consistency**: if both source operands are bound
//!    to variables with known concrete types (via a `src_types` map built
//!    on the fly), and those types differ, emit `TypeMismatch`.
//! 5. **Validate branch conditions**: for `jmp_if_true` / `jmp_if_false`,
//!    the first source must be typed `"bool"` if typed at all.
//!
//! # Note on `src_types` propagation
//!
//! This pass builds a simple SSA map `dest → type_hint` as it walks
//! instructions.  This lets it look up the type of a source variable when
//! checking a later instruction.  The map only contains concrete types;
//! `"any"` is excluded.

use std::collections::HashMap;

use interpreter_ir::module::IIRModule;
use interpreter_ir::opcodes::{is_concrete_type, DYNAMIC_TYPE};
use interpreter_ir::instr::Operand;

use crate::errors::{ErrorKind, TypeCheckError};
use crate::report::TypeCheckReport;

// ---------------------------------------------------------------------------
// Opcode classification helpers
// ---------------------------------------------------------------------------

/// Data-flow ops (have a destination register and a meaningful type).
///
/// Control-flow ops are excluded because their "type" is about the branch
/// predicate, not a produced value, so they don't count toward the typed
/// fraction.
fn is_data_op(op: &str) -> bool {
    !matches!(
        op,
        "jmp" | "jmp_if_true" | "jmp_if_false" | "label" | "ret_void"
    )
}

/// Binary arithmetic/bitwise/comparison ops that require operand-type consistency.
fn is_binary_data_op(op: &str) -> bool {
    matches!(
        op,
        "add" | "sub" | "mul" | "div" | "mod"
            | "and" | "or" | "xor" | "shl" | "shr"
            | "cmp_eq" | "cmp_ne" | "cmp_lt" | "cmp_le" | "cmp_gt" | "cmp_ge"
    )
}

/// Conditional branch ops that require a bool condition.
fn is_conditional_branch(op: &str) -> bool {
    matches!(op, "jmp_if_true" | "jmp_if_false")
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Validate the type annotations in `module` without modifying it.
///
/// Returns a [`TypeCheckReport`] describing the module's typing tier,
/// any errors found, and any warnings.  The `inferred_types` field of the
/// report is always empty — run [`crate::infer::infer_types_mut`] first
/// if you want inference.
///
/// # Example
///
/// ```
/// use interpreter_ir::module::IIRModule;
/// use interpreter_ir::function::IIRFunction;
/// use interpreter_ir::instr::{IIRInstr, Operand};
/// use iir_type_checker::check::check_module;
/// use iir_type_checker::tier::TypingTier;
///
/// // Fully-typed module: one const of type u8.
/// let fn_ = IIRFunction::new(
///     "main", vec![], "void",
///     vec![
///         IIRInstr::new("const", Some("x".into()), vec![Operand::Int(5)], "u8"),
///         IIRInstr::new("ret_void", None, vec![], "void"),
///     ],
/// );
/// let mut module = IIRModule::new("test", "tetrad");
/// module.functions.push(fn_);
/// let report = check_module(&module);
/// assert!(report.ok());
/// assert_eq!(report.tier, TypingTier::FullyTyped);
/// ```
pub fn check_module(module: &IIRModule) -> TypeCheckReport {
    let mut errors: Vec<TypeCheckError> = Vec::new();
    let mut total_data_instrs: usize = 0;
    let mut typed_data_instrs: usize = 0;

    for func in &module.functions {
        // SSA map: dest variable → concrete type_hint (only concrete types stored).
        let mut src_types: HashMap<String, String> = HashMap::new();

        for (idx, instr) in func.instructions.iter().enumerate() {
            let op = instr.op.as_str();
            let type_hint = instr.type_hint.as_str();

            // ── Count data-flow instructions ──────────────────────────────
            if is_data_op(op) {
                if instr.dest.is_some() {
                    total_data_instrs += 1;
                    if is_concrete_type(type_hint) {
                        typed_data_instrs += 1;
                    }
                }
            }

            // ── Validate conditional-branch condition is bool (runs even for
            //    "any" branch instructions because the first *source* variable
            //    may be concretely typed) ──────────────────────────────────
            if is_conditional_branch(op) && !instr.srcs.is_empty() {
                let t = operand_type(&instr.srcs[0], &src_types);
                if let Some(cond_type) = t {
                    if cond_type != "bool" {
                        errors.push(TypeCheckError {
                            fn_name: func.name.clone(),
                            instr_idx: idx,
                            kind: ErrorKind::ConditionNotBool,
                            message: format!(
                                "op '{}': condition operand has type '{}', expected 'bool'",
                                op, cond_type
                            ),
                        });
                    }
                }
            }

            // ── Skip untyped instructions — nothing more to validate ───────
            if type_hint == DYNAMIC_TYPE {
                continue;
            }

            // ── Validate the type_hint string itself ──────────────────────
            if !is_concrete_type(type_hint) && type_hint != "void" {
                errors.push(TypeCheckError {
                    fn_name: func.name.clone(),
                    instr_idx: idx,
                    kind: ErrorKind::InvalidType,
                    message: format!(
                        "op '{}': type_hint '{}' is not 'any', 'void', or a concrete type",
                        op, type_hint
                    ),
                });
                // Don't attempt further checks — type is broken.
                continue;
            }

            // ── Record the destination type for downstream instructions ───
            if let Some(dest) = &instr.dest {
                if is_concrete_type(type_hint) {
                    src_types.insert(dest.clone(), type_hint.to_string());
                }
            }

            // ── Validate binary-op operand consistency ────────────────────
            if is_binary_data_op(op) && instr.srcs.len() >= 2 {
                let t0 = operand_type(&instr.srcs[0], &src_types);
                let t1 = operand_type(&instr.srcs[1], &src_types);
                if let (Some(a), Some(b)) = (t0, t1) {
                    if a != b {
                        errors.push(TypeCheckError {
                            fn_name: func.name.clone(),
                            instr_idx: idx,
                            kind: ErrorKind::TypeMismatch,
                            message: format!(
                                "op '{}': src[0] is '{}' but src[1] is '{}' — types must agree",
                                op, a, b
                            ),
                        });
                    }
                }
            }

        }
    }

    let typed_fraction = if total_data_instrs == 0 {
        0.0_f32
    } else {
        typed_data_instrs as f32 / total_data_instrs as f32
    };

    let mut report = TypeCheckReport::new(typed_fraction);
    report.errors = errors;
    report
}

/// Compute the typing tier fractions without producing errors (available for
/// internal use by future modules in this crate).
#[allow(dead_code)]
pub(crate) fn measure_tier(module: &IIRModule) -> f32 {
    let mut total = 0usize;
    let mut typed = 0usize;
    for func in &module.functions {
        for instr in &func.instructions {
            if is_data_op(instr.op.as_str()) && instr.dest.is_some() {
                total += 1;
                if is_concrete_type(instr.type_hint.as_str()) {
                    typed += 1;
                }
            }
        }
    }
    if total == 0 { 0.0 } else { typed as f32 / total as f32 }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Look up the concrete type of an operand in the SSA map.
///
/// Returns `None` if:
/// - The operand is an immediate (not a variable reference).
/// - The operand is a variable whose type is not yet in the map (untyped).
fn operand_type<'a>(
    operand: &Operand,
    src_types: &'a HashMap<String, String>,
) -> Option<&'a str> {
    match operand {
        Operand::Var(name) => src_types.get(name.as_str()).map(String::as_str),
        Operand::Int(_) => None,    // immediates carry no SSA type
        Operand::Float(_) => None,
        Operand::Bool(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::function::IIRFunction;
    use interpreter_ir::instr::{IIRInstr, Operand};

    fn make_module(instrs: Vec<IIRInstr>) -> IIRModule {
        let fn_ = IIRFunction::new("main", vec![], "void", instrs);
        let mut m = IIRModule::new("test", "tetrad");
        m.functions.push(fn_);
        m
    }

    // ── Typed-fraction tests ──────────────────────────────────────────────

    #[test]
    fn empty_module_is_untyped() {
        let module = IIRModule::new("empty", "tetrad");
        let report = check_module(&module);
        assert_eq!(report.tier, crate::tier::TypingTier::Untyped);
        assert!(report.ok());
    }

    #[test]
    fn ret_void_only_module_is_untyped() {
        // ret_void has no dest; it should not count in the denominator.
        let module = make_module(vec![
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
        let report = check_module(&module);
        assert_eq!(report.tier, crate::tier::TypingTier::Untyped);
        assert!(report.ok());
    }

    #[test]
    fn single_typed_const_is_fully_typed() {
        let module = make_module(vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(1)], "u8"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
        let report = check_module(&module);
        assert_eq!(report.tier, crate::tier::TypingTier::FullyTyped);
        assert!(report.ok());
    }

    #[test]
    fn mix_of_typed_and_untyped_is_partial() {
        let module = make_module(vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "u8"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(2)], "any"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
        let report = check_module(&module);
        assert!(matches!(report.tier, crate::tier::TypingTier::Partial(_)));
    }

    // ── Validation tests ──────────────────────────────────────────────────

    #[test]
    fn invalid_type_hint_produces_error() {
        let module = make_module(vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(1)], "integer"),
        ]);
        let report = check_module(&module);
        assert!(!report.ok());
        assert_eq!(report.errors[0].kind, ErrorKind::InvalidType);
    }

    #[test]
    fn mismatched_binary_op_types_produce_error() {
        // a: u8, b: f64 — add requires same types
        let module = make_module(vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "u8"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Float(1.0)], "f64"),
            IIRInstr::new("add", Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "u8"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
        let report = check_module(&module);
        assert!(!report.ok());
        assert_eq!(report.errors[0].kind, ErrorKind::TypeMismatch);
    }

    #[test]
    fn matching_binary_op_types_are_ok() {
        let module = make_module(vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "u8"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(2)], "u8"),
            IIRInstr::new("add", Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "u8"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
        let report = check_module(&module);
        assert!(report.ok(), "errors: {:?}", report.errors);
    }

    #[test]
    fn conditional_branch_on_non_bool_produces_error() {
        let module = make_module(vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("jmp_if_true",
                None,
                vec![Operand::Var("x".into()), Operand::Var("L".into())],
                "any"),
            IIRInstr::new("label", None, vec![Operand::Var("L".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
        let report = check_module(&module);
        assert!(!report.ok());
        assert_eq!(report.errors[0].kind, ErrorKind::ConditionNotBool);
    }

    #[test]
    fn conditional_branch_on_bool_is_ok() {
        let module = make_module(vec![
            IIRInstr::new("const", Some("cond".into()), vec![Operand::Bool(true)], "bool"),
            IIRInstr::new("jmp_if_true",
                None,
                vec![Operand::Var("cond".into()), Operand::Var("L".into())],
                "any"),
            IIRInstr::new("label", None, vec![Operand::Var("L".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
        let report = check_module(&module);
        assert!(report.ok(), "errors: {:?}", report.errors);
    }

    #[test]
    fn untyped_binary_op_is_not_an_error() {
        // When all srcs are "any", we can't validate — that's fine.
        let module = make_module(vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "any"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(2)], "any"),
            IIRInstr::new("add", Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "any"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
        let report = check_module(&module);
        assert!(report.ok());
    }
}
