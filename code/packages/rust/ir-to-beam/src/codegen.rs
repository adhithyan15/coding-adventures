//! `BEAMCodeGenerator` — LANG20 `CodeGenerator` adapter for the BEAM backend.
//!
//! This module provides the thin adapter that wires [`lower_ir_to_beam`] and
//! [`validate_for_beam`] into the [`codegen_core::codegen::CodeGenerator`]
//! protocol defined by LANG20.
//!
//! # Responsibilities
//!
//! | Method       | Delegates to                          |
//! |--------------|---------------------------------------|
//! | `name()`     | returns `"beam"` (stable identifier)  |
//! | `validate()` | [`validate_for_beam`]                 |
//! | `generate()` | [`lower_ir_to_beam`]                  |
//!
//! `generate()` panics if `validate()` would return errors — callers should
//! always validate first in production code.
//!
//! # Usage
//!
//! ```
//! use compiler_ir::{IrProgram, IrInstruction, IrOp};
//! use ir_to_beam::codegen::BEAMCodeGenerator;
//! use codegen_core::codegen::CodeGenerator;
//!
//! let gen = BEAMCodeGenerator::new("mymod");
//!
//! let mut prog = IrProgram::new("_start");
//! prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 0));
//!
//! assert!(gen.validate(&prog).is_empty());
//! let module = gen.generate(&prog);
//! assert_eq!(module.name, "mymod");
//! ```

use codegen_core::codegen::CodeGenerator;
use compiler_ir::IrProgram;

use crate::backend::{lower_ir_to_beam, validate_for_beam, BEAMBackendConfig};
use crate::encoder::BEAMModule;

// ===========================================================================
// BEAMCodeGenerator
// ===========================================================================

/// Implements the `CodeGenerator<IrProgram, BEAMModule>` protocol for BEAM.
///
/// Construct with [`BEAMCodeGenerator::new`], passing the Erlang module name
/// the generated `.beam` file should carry.
#[derive(Debug, Clone)]
pub struct BEAMCodeGenerator {
    config: BEAMBackendConfig,
}

impl BEAMCodeGenerator {
    /// Create a new generator that will emit a module named `module_name`.
    ///
    /// # Example
    ///
    /// ```
    /// use ir_to_beam::codegen::BEAMCodeGenerator;
    /// let gen = BEAMCodeGenerator::new("calc");
    /// ```
    pub fn new(module_name: impl Into<String>) -> Self {
        Self { config: BEAMBackendConfig { module_name: module_name.into() } }
    }

    /// Create a generator with the default module name `"main"`.
    pub fn default_module() -> Self {
        Self { config: BEAMBackendConfig::default() }
    }
}

impl CodeGenerator<IrProgram, BEAMModule> for BEAMCodeGenerator {
    /// Stable backend identifier — always `"beam"`.
    fn name(&self) -> &str {
        "beam"
    }

    /// Validate `ir` for BEAM lowering.
    ///
    /// Returns a list of human-readable error strings.
    /// An empty list means the program is safe to pass to `generate()`.
    fn validate(&self, ir: &IrProgram) -> Vec<String> {
        let mut errs = validate_for_beam(ir);
        if self.config.module_name.is_empty() {
            errs.push("module_name must not be empty".to_string());
        }
        errs
    }

    /// Lower `ir` to a [`BEAMModule`].
    ///
    /// # Panics
    ///
    /// Panics if `validate(ir)` would have returned errors.
    fn generate(&self, ir: &IrProgram) -> BEAMModule {
        lower_ir_to_beam(ir, &self.config)
            .unwrap_or_else(|e| panic!("BEAMCodeGenerator::generate called on invalid IR: {}", e))
    }
}

// ===========================================================================
// Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use codegen_core::codegen::CodeGenerator;
    use compiler_ir::{IrInstruction, IrOperand, IrOp, IrProgram};

    fn halt_prog() -> IrProgram {
        let mut p = IrProgram::new("_start");
        p.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 0));
        p
    }

    // Test 1: name() is "beam"
    #[test]
    fn name_is_beam() {
        let gen = BEAMCodeGenerator::new("test");
        assert_eq!(gen.name(), "beam");
    }

    // Test 2: validate returns [] for valid IR
    #[test]
    fn validate_valid_ir() {
        let gen = BEAMCodeGenerator::new("test");
        assert!(gen.validate(&halt_prog()).is_empty());
    }

    // Test 3: validate catches unsupported op
    #[test]
    fn validate_catches_load_byte() {
        let gen = BEAMCodeGenerator::new("test");
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadByte,
            vec![IrOperand::Register(0), IrOperand::Register(1), IrOperand::Register(2)],
            0,
        ));
        let errs = gen.validate(&prog);
        assert!(!errs.is_empty());
    }

    // Test 4: validate catches empty module name
    #[test]
    fn validate_catches_empty_module_name() {
        let gen = BEAMCodeGenerator::new("");
        let errs = gen.validate(&halt_prog());
        assert!(!errs.is_empty());
    }

    // Test 5: generate returns a BEAMModule with correct name
    #[test]
    fn generate_returns_correct_module_name() {
        let gen = BEAMCodeGenerator::new("calc");
        let module = gen.generate(&halt_prog());
        assert_eq!(module.name, "calc");
    }

    // Test 6: generate produces non-empty instructions
    #[test]
    fn generate_produces_instructions() {
        let gen = BEAMCodeGenerator::new("calc");
        let module = gen.generate(&halt_prog());
        assert!(!module.instructions.is_empty());
    }

    // Test 7: generate exports run/0
    #[test]
    fn generate_exports_run_zero() {
        let gen = BEAMCodeGenerator::new("calc");
        let module = gen.generate(&halt_prog());
        assert_eq!(module.exports.len(), 1);
        assert_eq!(module.exports[0].arity, 0);
    }

    // Test 8: default_module() uses "main"
    #[test]
    fn default_module_name() {
        let gen = BEAMCodeGenerator::default_module();
        let module = gen.generate(&halt_prog());
        assert_eq!(module.name, "main");
    }

    // Test 9: validate then generate round-trip
    #[test]
    fn validate_then_generate_round_trip() {
        let gen = BEAMCodeGenerator::new("roundtrip");
        let prog = halt_prog();
        assert!(gen.validate(&prog).is_empty());
        let module = gen.generate(&prog);
        assert_eq!(module.name, "roundtrip");
    }

    // Test 10: arithmetic program validates and generates without panic
    #[test]
    fn arithmetic_program_generates() {
        let gen = BEAMCodeGenerator::new("arith");
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(0), IrOperand::Immediate(10)],
            0,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(1), IrOperand::Immediate(5)],
            1,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::Add,
            vec![IrOperand::Register(2), IrOperand::Register(0), IrOperand::Register(1)],
            2,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 3));
        assert!(gen.validate(&prog).is_empty());
        let module = gen.generate(&prog);
        assert!(!module.instructions.is_empty());
    }

    // Test 11: encode_beam on generated module produces valid FOR1 bytes
    #[test]
    fn generate_then_encode_valid_for1() {
        use crate::encoder::encode_beam;
        let gen = BEAMCodeGenerator::new("enc");
        let module = gen.generate(&halt_prog());
        let bytes = encode_beam(&module);
        assert_eq!(&bytes[0..4], b"FOR1");
    }
}
