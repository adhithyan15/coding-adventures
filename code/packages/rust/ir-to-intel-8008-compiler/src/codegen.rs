//! `Intel8008CodeGenerator` — LANG20 `CodeGenerator<IrProgram, String>` adapter.
//!
//! This module wraps `IrValidator` and `IrToIntel8008Compiler` in the LANG20
//! `CodeGenerator` protocol so callers can use any backend interchangeably via
//! the `codegen_core::codegen::CodeGenerator` trait.
//!
//! ## Example
//!
//! ```rust
//! use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};
//! use codegen_core::codegen::CodeGenerator;
//! use ir_to_intel_8008_compiler::codegen::Intel8008CodeGenerator;
//!
//! let mut prog = IrProgram::new("_start");
//! prog.add_instruction(IrInstruction::new(
//!     IrOp::LoadImm,
//!     vec![IrOperand::Register(1), IrOperand::Immediate(7)],
//!     1,
//! ));
//! prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 2));
//!
//! let gen = Intel8008CodeGenerator;
//! assert!(gen.validate(&prog).is_empty(), "valid program should have no errors");
//! let asm = gen.generate(&prog);
//! assert!(asm.contains("MVI  C, 7"));
//! assert!(asm.contains("HLT"));
//! ```

use codegen_core::codegen::CodeGenerator;
use compiler_ir::IrProgram;

use crate::{IrToIntel8008Compiler, IrValidator};

// ── Intel8008CodeGenerator ────────────────────────────────────────────────────

/// LANG20 `CodeGenerator<IrProgram, String>` adapter for the Intel 8008 backend.
///
/// Wraps the existing `IrValidator` and `IrToIntel8008Compiler` so they
/// participate in the shared `CodeGenerator` protocol.  Assembly is returned
/// as an `Intel 8008` text-assembly `String`.
///
/// ## Usage
///
/// ```rust
/// use codegen_core::codegen::CodeGenerator;
/// use ir_to_intel_8008_compiler::codegen::Intel8008CodeGenerator;
/// use compiler_ir::IrProgram;
///
/// let gen = Intel8008CodeGenerator;
/// assert_eq!(gen.name(), "intel8008");
/// let prog = IrProgram::new("_start");
/// // validate returns errors here (no HALT), but the type is correct.
/// let _errors: Vec<String> = gen.validate(&prog);
/// ```
pub struct Intel8008CodeGenerator;

impl CodeGenerator<IrProgram, String> for Intel8008CodeGenerator {
    /// Stable backend name used for registry lookups and debug output.
    fn name(&self) -> &str {
        "intel8008"
    }

    /// Validate `ir` for the Intel 8008 target.
    ///
    /// Returns a `Vec<String>` where each element is a human-readable error
    /// message.  An empty vec means the program is valid for this backend.
    ///
    /// Delegates to `IrValidator` and converts `IrValidationError` → `String`
    /// via the `Display` impl.
    fn validate(&self, ir: &IrProgram) -> Vec<String> {
        IrValidator
            .validate(ir)
            .into_iter()
            .map(|e| e.to_string())
            .collect()
    }

    /// Compile `ir` to Intel 8008 assembly text.
    ///
    /// # Panics
    ///
    /// Panics if `validate(ir)` returns any errors.  Well-behaved callers
    /// always call `validate` first (or rely on the fact that the compiler
    /// itself calls `validate` internally and panics on invalid input).
    fn generate(&self, ir: &IrProgram) -> String {
        IrToIntel8008Compiler
            .compile(ir)
            .unwrap_or_else(|errs| {
                panic!(
                    "Intel8008CodeGenerator::generate called on invalid IR \
                     (call validate() first):\n{}",
                    errs.iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            })
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use codegen_core::codegen::CodeGenerator;
    use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    /// Minimal valid program: `LOAD_IMM v1, 1 ; HALT`
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

    // ------------------------------------------------------------------
    // name()
    // ------------------------------------------------------------------

    #[test]
    fn test_name_is_intel8008() {
        assert_eq!(Intel8008CodeGenerator.name(), "intel8008");
    }

    // ------------------------------------------------------------------
    // validate()
    // ------------------------------------------------------------------

    #[test]
    fn test_validate_valid_program_returns_empty() {
        let prog = minimal_prog();
        let errors = Intel8008CodeGenerator.validate(&prog);
        assert!(
            errors.is_empty(),
            "expected no errors for minimal valid prog, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validate_bad_opcode_returns_error() {
        // LOAD_WORD is forbidden on the 8008 (no 16-bit memory ops).
        use compiler_ir::IrDataDecl;
        let mut prog = IrProgram::new("_start");
        prog.data.push(IrDataDecl {
            label: "buf".to_string(),
            size: 4,
            init: 0,
        });
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadWord,
            vec![
                IrOperand::Register(1),
                IrOperand::Label("buf".to_string()),
            ],
            1,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 2));
        let errors = Intel8008CodeGenerator.validate(&prog);
        assert!(
            !errors.is_empty(),
            "expected errors for LOAD_WORD on 8008, got none"
        );
    }

    #[test]
    fn test_validate_too_many_registers_returns_error() {
        // Using v6 (index 6) exceeds the 8008's 6-register limit.
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(6), IrOperand::Immediate(1)],
            1,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 2));
        let errors = Intel8008CodeGenerator.validate(&prog);
        assert!(
            !errors.is_empty(),
            "expected errors for v6 (out of range), got none"
        );
    }

    #[test]
    fn test_validate_immediate_out_of_range_returns_error() {
        // 256 does not fit in u8.
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(1), IrOperand::Immediate(256)],
            1,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 2));
        let errors = Intel8008CodeGenerator.validate(&prog);
        assert!(
            !errors.is_empty(),
            "expected errors for immediate 256, got none"
        );
    }

    // ------------------------------------------------------------------
    // generate()
    // ------------------------------------------------------------------

    #[test]
    fn test_generate_valid_program_returns_string() {
        let prog = minimal_prog();
        let asm = Intel8008CodeGenerator.generate(&prog);
        assert!(!asm.is_empty(), "expected non-empty assembly");
    }

    #[test]
    fn test_generate_does_not_raise_on_valid_ir() {
        // Simply verify generate() does not panic on a valid program.
        let prog = minimal_prog();
        let _asm = Intel8008CodeGenerator.generate(&prog);
    }

    #[test]
    fn test_generate_returns_correct_assembly_type() {
        let prog = minimal_prog();
        let asm = Intel8008CodeGenerator.generate(&prog);
        // String is the expected Assembly type; check it contains ORG header.
        assert!(
            asm.contains("ORG"),
            "expected ORG directive in Intel 8008 output, got: {asm}"
        );
    }

    #[test]
    fn test_generate_contains_halt() {
        let prog = minimal_prog();
        let asm = Intel8008CodeGenerator.generate(&prog);
        assert!(
            asm.contains("HLT"),
            "expected HLT in Intel 8008 output, got: {asm}"
        );
    }

    // ------------------------------------------------------------------
    // Round-trip: validate then generate
    // ------------------------------------------------------------------

    #[test]
    fn test_round_trip_validate_then_generate() {
        let prog = minimal_prog();
        let errors = Intel8008CodeGenerator.validate(&prog);
        assert!(errors.is_empty(), "validation failed: {:?}", errors);
        let asm = Intel8008CodeGenerator.generate(&prog);
        assert!(asm.contains("MVI  C, 1"), "expected MVI C, 1 in: {asm}");
        assert!(asm.contains("HLT"), "expected HLT in: {asm}");
    }

    #[test]
    fn test_round_trip_with_add_imm() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(2), IrOperand::Immediate(10)],
            1,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::AddImm,
            vec![
                IrOperand::Register(2), // dst (D)
                IrOperand::Register(2), // src (D)
                IrOperand::Immediate(5),
            ],
            2,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 3));
        let errors = Intel8008CodeGenerator.validate(&prog);
        assert!(errors.is_empty(), "validation errors: {:?}", errors);
        let asm = Intel8008CodeGenerator.generate(&prog);
        assert!(asm.contains("MVI  D, 10"), "expected MVI D, 10 in: {asm}");
        assert!(asm.contains("ADI  5"), "expected ADI  5 in: {asm}");
    }
}
