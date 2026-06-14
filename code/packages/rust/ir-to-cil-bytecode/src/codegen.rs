//! `CILCodeGenerator` — LANG20 `CodeGenerator<IrProgram, CILProgramArtifact>` adapter.
//!
//! Wraps `validate_for_clr` and `lower_ir_to_cil_bytecode` in the shared
//! `codegen_core::codegen::CodeGenerator` protocol so callers can use any
//! backend interchangeably.
//!
//! ## Example
//!
//! ```
//! use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};
//! use codegen_core::codegen::CodeGenerator;
//! use ir_to_cil_bytecode::codegen::CILCodeGenerator;
//!
//! let mut prog = IrProgram::new("_start");
//! prog.add_instruction(IrInstruction::new(
//!     IrOp::LoadImm,
//!     vec![IrOperand::Register(1), IrOperand::Immediate(7)],
//!     1,
//! ));
//! prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 2));
//!
//! let gen = CILCodeGenerator::new();
//! assert!(gen.validate(&prog).is_empty());
//! let artifact = gen.generate(&prog);
//! assert!(!artifact.methods[0].body.is_empty());
//! ```

use codegen_core::codegen::CodeGenerator;
use compiler_ir::IrProgram;

use crate::backend::{lower_ir_to_cil_bytecode, validate_for_clr, CILBackendConfig, CILProgramArtifact};

// ── CILCodeGenerator ──────────────────────────────────────────────────────

/// LANG20 `CodeGenerator<IrProgram, CILProgramArtifact>` adapter for the
/// CLR (Common Language Runtime) backend.
///
/// Wraps `validate_for_clr` and `lower_ir_to_cil_bytecode` so the CLR backend
/// participates in the shared code-generator protocol.
///
/// Assembly is returned as a `CILProgramArtifact` — a structured multi-method
/// artifact ready for the CLR simulator or a PE-file packager.
#[derive(Debug, Default)]
pub struct CILCodeGenerator {
    config: Option<CILBackendConfig>,
}

impl CILCodeGenerator {
    /// Create a new `CILCodeGenerator` with default configuration.
    pub fn new() -> Self {
        Self { config: None }
    }

    /// Create a `CILCodeGenerator` with a custom backend configuration.
    pub fn with_config(config: CILBackendConfig) -> Self {
        Self { config: Some(config) }
    }
}

impl CodeGenerator<IrProgram, CILProgramArtifact> for CILCodeGenerator {
    /// Stable backend name used for registry lookups and debug output.
    fn name(&self) -> &str {
        "cil"
    }

    /// Validate `ir` for the CLR target.
    ///
    /// Returns a `Vec<String>` of error messages; empty = valid.
    fn validate(&self, ir: &IrProgram) -> Vec<String> {
        validate_for_clr(ir)
    }

    /// Compile `ir` to a `CILProgramArtifact`.
    ///
    /// # Panics
    ///
    /// Panics if `validate(ir)` would return errors.  Callers must validate
    /// first or ensure their IR is valid for the CLR target.
    fn generate(&self, ir: &IrProgram) -> CILProgramArtifact {
        let config = self.config.clone();
        lower_ir_to_cil_bytecode(ir, config, None)
            .unwrap_or_else(|e| {
                panic!(
                    "CILCodeGenerator::generate called on invalid IR \
                     (call validate() first): {}",
                    e
                )
            })
    }
}

// ===========================================================================
// Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use codegen_core::codegen::CodeGenerator;
    use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};

    fn minimal_prog() -> IrProgram {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(1), IrOperand::Immediate(1)],
            1,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 2));
        prog
    }

    #[test]
    fn test_name_is_cil() {
        assert_eq!(CILCodeGenerator::new().name(), "cil");
    }

    #[test]
    fn test_validate_valid_program_returns_empty() {
        let prog = minimal_prog();
        assert!(CILCodeGenerator::new().validate(&prog).is_empty());
    }

    #[test]
    fn test_validate_bad_opcode_returns_error() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::Syscall,
            vec![IrOperand::Immediate(99)],
            1,
        ));
        let errors = CILCodeGenerator::new().validate(&prog);
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_validate_bad_immediate_returns_error() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(0), IrOperand::Immediate(i64::MAX)],
            1,
        ));
        let errors = CILCodeGenerator::new().validate(&prog);
        assert!(!errors.is_empty());
        assert!(errors[0].contains("int32 range"));
    }

    #[test]
    fn test_generate_valid_program_returns_artifact() {
        let prog = minimal_prog();
        let artifact = CILCodeGenerator::new().generate(&prog);
        assert!(!artifact.methods.is_empty());
        assert!(!artifact.methods[0].body.is_empty());
    }

    #[test]
    fn test_generate_does_not_panic_on_valid_ir() {
        let prog = minimal_prog();
        let _artifact = CILCodeGenerator::new().generate(&prog);
    }

    #[test]
    fn test_generate_returns_correct_assembly_type() {
        let prog = minimal_prog();
        let artifact = CILCodeGenerator::new().generate(&prog);
        // CILProgramArtifact contains methods, helper_specs, data_offsets
        assert_eq!(artifact.entry_label, "_start");
        assert_eq!(artifact.helper_specs.len(), 5);
    }

    #[test]
    fn test_generate_body_contains_ret() {
        let prog = minimal_prog();
        let artifact = CILCodeGenerator::new().generate(&prog);
        let body = &artifact.methods[0].body;
        assert!(body.contains(&0x2A), "expected ret (0x2A) in: {body:?}");
    }

    #[test]
    fn test_round_trip_validate_then_generate() {
        let prog = minimal_prog();
        let gen = CILCodeGenerator::new();
        let errors = gen.validate(&prog);
        assert!(errors.is_empty(), "validation errors: {:?}", errors);
        let artifact = gen.generate(&prog);
        assert!(!artifact.methods[0].body.is_empty());
    }

    #[test]
    fn test_round_trip_with_add() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(1), IrOperand::Immediate(3)],
            1,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(2), IrOperand::Immediate(4)],
            2,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::Add,
            vec![
                IrOperand::Register(1),
                IrOperand::Register(1),
                IrOperand::Register(2),
            ],
            3,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 4));
        let gen = CILCodeGenerator::new();
        assert!(gen.validate(&prog).is_empty());
        let artifact = gen.generate(&prog);
        assert!(artifact.methods[0].body.contains(&0x58)); // add opcode
    }
}
