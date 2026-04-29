//! # Validator — pre-lowering safety checks for a CIR instruction list.
//!
//! `validate_cir_for_lowering` detects patterns that the lowerer cannot handle
//! **before** attempting any translation. This provides clear, actionable error
//! messages rather than cryptic mid-lowering panics.
//!
//! ## Checks performed
//!
//! | Check | Why |
//! |---|---|
//! | Empty list | Nothing to lower — always a bug in the caller. |
//! | `call_runtime` present | Runtime dispatch is unsupported in v1. |
//! | `io_in` / `io_out` present | I/O is backend-specific; no IR equivalent. |
//! | `type == "any"` on data ops | Specialisation failed to resolve the type. |
//!
//! ## Usage
//!
//! ```
//! use jit_core::CIRInstr;
//! use cir_to_compiler_ir::validate_cir_for_lowering;
//!
//! let instrs = vec![
//!     CIRInstr::new("ret_void", None::<String>, vec![], "void"),
//! ];
//! let errors = validate_cir_for_lowering(&instrs);
//! assert!(errors.is_empty(), "should be valid: {:?}", errors);
//! ```

use jit_core::CIRInstr;

// ===========================================================================
// Control-flow ops — excluded from the `type == "any"` check
// ===========================================================================

/// These ops do not carry a meaningful data type. Checking `ty == "any"` on
/// them would produce false positives (they legitimately carry "void" or "any"
/// as a placeholder type in some CIR emitters).
const CONTROL_FLOW_OPS: &[&str] = &[
    "label",
    "jmp",
    "jmp_if_true",
    "jmp_if_false",
    "br_true_bool",
    "br_false_bool",
    "ret_void",
    "type_assert",
    "call",
    "tetrad.move",
];

// ===========================================================================
// Public API
// ===========================================================================

/// Validate a CIR instruction list before lowering.
///
/// Returns a list of human-readable error strings. An empty list means the
/// instruction list is safe to pass to `lower_cir_to_ir_program`.
///
/// # Arguments
///
/// * `instrs` — the CIR instruction list produced by `jit_core::specialise`.
///
/// # Example — valid program passes
///
/// ```
/// use jit_core::{CIRInstr, CIROperand};
/// use cir_to_compiler_ir::validate_cir_for_lowering;
///
/// let instrs = vec![
///     CIRInstr::new("const_i32", Some("x".to_string()), vec![CIROperand::Int(1)], "i32"),
///     CIRInstr::new("ret_void", None::<String>, vec![], "void"),
/// ];
/// assert!(validate_cir_for_lowering(&instrs).is_empty());
/// ```
///
/// # Example — `call_runtime` is rejected
///
/// ```
/// use jit_core::CIRInstr;
/// use cir_to_compiler_ir::validate_cir_for_lowering;
///
/// let instrs = vec![
///     CIRInstr::new("call_runtime", None::<String>, vec![], "void"),
/// ];
/// let errors = validate_cir_for_lowering(&instrs);
/// assert!(!errors.is_empty());
/// assert!(errors[0].contains("call_runtime"));
/// ```
pub fn validate_cir_for_lowering(instrs: &[CIRInstr]) -> Vec<String> {
    let mut errors = Vec::new();

    // ── Check 1: empty list ─────────────────────────────────────────────────
    //
    // An empty instruction list has no entry point and no HALT. Every valid
    // CIR function emits at least one instruction.
    if instrs.is_empty() {
        errors.push("instruction list is empty — nothing to lower".to_string());
        return errors; // no point checking further
    }

    for instr in instrs {
        // ── Check 2: call_runtime ───────────────────────────────────────────
        //
        // `call_runtime` invokes heap allocation, garbage collection, or other
        // VM services that have no architecture-independent IR encoding in v1.
        // A future LANG will add a `RUNTIME_CALL` IR opcode.
        if instr.op == "call_runtime" {
            errors.push(format!(
                "call_runtime is not supported in v1 IR lowering \
                 (dest={:?}); use a specialised backend instead",
                instr.dest
            ));
        }

        // ── Check 3: io_in / io_out ─────────────────────────────────────────
        //
        // `io_in` and `io_out` are machine-specific I/O operations (ports on
        // x86, MMIO on embedded targets). There is no target-independent IR
        // encoding.
        if instr.op == "io_in" || instr.op == "io_out" {
            errors.push(format!(
                "I/O op '{}' is not supported in v1 IR lowering; \
                 I/O must be handled by a platform-specific backend",
                instr.op
            ));
        }

        // ── Check 4: type == "any" on data ops ──────────────────────────────
        //
        // If specialisation succeeded, every data-producing instruction has a
        // concrete type (i32, u8, bool, etc.). The sentinel "any" means the
        // specialiser could not determine the type — the lowering would be
        // incorrect because we cannot choose the right IR opcode.
        if instr.ty == "any" && !CONTROL_FLOW_OPS.contains(&instr.op.as_str()) {
            errors.push(format!(
                "instruction op='{}' has unresolved type 'any'; \
                 specialisation must resolve all types before lowering",
                instr.op
            ));
        }
    }

    errors
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use jit_core::{CIRInstr, CIROperand};

    fn ret_void() -> CIRInstr {
        CIRInstr::new("ret_void", None::<String>, vec![], "void")
    }

    #[test]
    fn test_valid_single_instr_passes() {
        let instrs = vec![ret_void()];
        assert!(validate_cir_for_lowering(&instrs).is_empty());
    }

    #[test]
    fn test_empty_list_returns_error() {
        let errors = validate_cir_for_lowering(&[]);
        assert!(!errors.is_empty());
        assert!(errors[0].contains("empty"));
    }

    #[test]
    fn test_call_runtime_returns_error() {
        let instrs = vec![
            CIRInstr::new("call_runtime", None::<String>, vec![], "void"),
        ];
        let errors = validate_cir_for_lowering(&instrs);
        assert!(!errors.is_empty());
        assert!(errors[0].contains("call_runtime"));
    }

    #[test]
    fn test_io_in_returns_error() {
        let instrs = vec![CIRInstr::new("io_in", None::<String>, vec![], "void")];
        let errors = validate_cir_for_lowering(&instrs);
        assert!(!errors.is_empty());
        assert!(errors[0].contains("io_in"));
    }

    #[test]
    fn test_io_out_returns_error() {
        let instrs = vec![CIRInstr::new("io_out", None::<String>, vec![], "void")];
        let errors = validate_cir_for_lowering(&instrs);
        assert!(!errors.is_empty());
        assert!(errors[0].contains("io_out"));
    }

    #[test]
    fn test_any_type_on_data_op_returns_error() {
        let instrs = vec![
            CIRInstr::new("add_any", Some("v".to_string()), vec![], "any"),
        ];
        let errors = validate_cir_for_lowering(&instrs);
        assert!(!errors.is_empty());
        assert!(errors[0].contains("any"));
    }

    #[test]
    fn test_control_flow_ops_exempt_from_any_type_check() {
        // These ops may legitimately carry type "any" in some emitters.
        for op in CONTROL_FLOW_OPS {
            let instrs = vec![
                CIRInstr::new(*op, None::<String>, vec![], "any"),
                ret_void(),
            ];
            let errors: Vec<_> = validate_cir_for_lowering(&instrs)
                .into_iter()
                .filter(|e| e.contains("unresolved type"))
                .collect();
            assert!(
                errors.is_empty(),
                "op '{}' should be exempt from any-type check but got: {:?}",
                op, errors
            );
        }
    }

    #[test]
    fn test_valid_program_passes() {
        let instrs = vec![
            CIRInstr::new("const_i32", Some("x".to_string()), vec![CIROperand::Int(42)], "i32"),
            CIRInstr::new("ret_void", None::<String>, vec![], "void"),
        ];
        assert!(validate_cir_for_lowering(&instrs).is_empty());
    }

    #[test]
    fn test_multiple_errors_collected() {
        let instrs = vec![
            CIRInstr::new("call_runtime", None::<String>, vec![], "void"),
            CIRInstr::new("io_in", None::<String>, vec![], "void"),
        ];
        let errors = validate_cir_for_lowering(&instrs);
        assert!(errors.len() >= 2, "expected multiple errors, got: {:?}", errors);
    }
}
