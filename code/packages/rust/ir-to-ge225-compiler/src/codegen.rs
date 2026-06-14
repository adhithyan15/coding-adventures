//! `GE225CodeGenerator` — a [`CodeGenerator`] adapter for the GE-225 backend.
//!
//! This module wraps [`compile_to_ge225`] and [`validate_for_ge225`] in the
//! `CodeGenerator<IrProgram, CompileResult>` protocol defined in `codegen-core`.
//! It is the standard entry point for generic multi-backend pipelines.
//!
//! # Example
//!
//! ```
//! use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};
//! use ir_to_ge225_compiler::codegen::GE225CodeGenerator;
//! use codegen_core::CodeGenerator;
//!
//! let mut prog = IrProgram::new("_start");
//! prog.add_instruction(IrInstruction::new(
//!     IrOp::LoadImm,
//!     vec![IrOperand::Register(0), IrOperand::Immediate(1)],
//!     1,
//! ));
//! prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 2));
//!
//! let gen = GE225CodeGenerator;
//! assert!(gen.validate(&prog).is_empty());
//! let result = gen.generate(&prog);
//! assert_eq!(result.binary.len() % 3, 0);
//! ```

use codegen_core::CodeGenerator;
use compiler_ir::IrProgram;

use crate::{CompileResult, compile_to_ge225, validate_for_ge225};

/// `CodeGenerator` adapter for the GE-225 backend.
///
/// Delegates to [`validate_for_ge225`] and [`compile_to_ge225`] from this
/// crate. The `Assembly` type is [`CompileResult`], which bundles the packed
/// binary, halt address, data base, and label map — everything downstream
/// code needs to load and run the program on a GE-225 simulator.
pub struct GE225CodeGenerator;

impl CodeGenerator<IrProgram, CompileResult> for GE225CodeGenerator {
    /// Short identifier for this backend. Used by `CodeGeneratorRegistry`.
    fn name(&self) -> &str {
        "ge225"
    }

    /// Check whether `ir` is compatible with the V1 GE-225 backend.
    ///
    /// Runs the four pre-flight rules from [`validate_for_ge225`]:
    /// 1. Every opcode must be in the V1 supported set.
    /// 2. `LOAD_IMM` / `ADD_IMM` constants must fit in 20-bit signed range.
    /// 3. `SYSCALL` number must be 1.
    /// 4. `AND_IMM` immediate must be 1.
    ///
    /// Returns an empty `Vec` when the program is valid.
    fn validate(&self, ir: &IrProgram) -> Vec<String> {
        validate_for_ge225(ir)
    }

    /// Compile `ir` to a GE-225 binary image.
    ///
    /// Calls [`validate`](Self::validate) internally. Panics if the program is
    /// invalid — callers should call [`validate`](Self::validate) first and
    /// check for errors before calling this method.
    fn generate(&self, ir: &IrProgram) -> CompileResult {
        compile_to_ge225(ir).expect(
            "GE225CodeGenerator::generate called on an invalid IrProgram; \
             call validate() first"
        )
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use codegen_core::CodeGenerator;
    use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};

    fn minimal_prog() -> IrProgram {
        let mut p = IrProgram::new("_start");
        p.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(0), IrOperand::Immediate(1)],
            1,
        ));
        p.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 2));
        p
    }

    fn invalid_prog() -> IrProgram {
        let mut p = IrProgram::new("_start");
        // LoadByte is not supported in the V1 GE-225 backend.
        p.add_instruction(IrInstruction::new(
            IrOp::LoadByte,
            vec![IrOperand::Register(0), IrOperand::Register(1)],
            1,
        ));
        p
    }

    #[test]
    fn test_name_is_ge225() {
        assert_eq!(GE225CodeGenerator.name(), "ge225");
    }

    #[test]
    fn test_validate_valid_program_returns_empty() {
        let gen = GE225CodeGenerator;
        assert!(gen.validate(&minimal_prog()).is_empty());
    }

    #[test]
    fn test_validate_invalid_program_returns_errors() {
        let gen = GE225CodeGenerator;
        let errors = gen.validate(&invalid_prog());
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_generate_returns_compile_result() {
        let gen = GE225CodeGenerator;
        let result = gen.generate(&minimal_prog());
        assert_eq!(result.binary.len() % 3, 0);
        assert!(result.halt_address > 0);
        assert_eq!(result.data_base, result.halt_address + 1);
    }

    #[test]
    fn test_generate_binary_is_non_empty() {
        let gen = GE225CodeGenerator;
        let result = gen.generate(&minimal_prog());
        // At minimum: TON + LOAD_IMM (2) + HALT (1) + halt_stub (1) + spill (1) + const (1) = 7 words = 21 bytes
        assert!(result.binary.len() >= 21);
    }

    #[test]
    fn test_satisfies_code_generator_trait() {
        // Compile-time check: GE225CodeGenerator implements CodeGenerator<_, _>.
        fn accept<IR, A, G: CodeGenerator<IR, A>>(_: &G) {}
        accept::<IrProgram, CompileResult, GE225CodeGenerator>(&GE225CodeGenerator);
    }

    #[test]
    fn test_round_trip_validate_then_generate() {
        let gen = GE225CodeGenerator;
        let prog = minimal_prog();
        let errors = gen.validate(&prog);
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
        let result = gen.generate(&prog);
        assert!(!result.binary.is_empty());
    }

    #[test]
    fn test_halt_address_stored_in_result() {
        let gen = GE225CodeGenerator;
        let result = gen.generate(&minimal_prog());
        // The halt stub must be at a non-zero address (word 0 is TON).
        assert!(result.halt_address > 0);
    }

    #[test]
    fn test_constant_overflow_fails_validation() {
        let gen = GE225CodeGenerator;
        let mut p = IrProgram::new("_start");
        p.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(0), IrOperand::Immediate(1_000_000)],
            1,
        ));
        let errors = gen.validate(&p);
        assert!(!errors.is_empty());
        assert!(errors[0].contains("overflow") || errors[0].contains("overflows"));
    }
}
