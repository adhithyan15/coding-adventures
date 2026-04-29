//! `Intel4004CodeGenerator` — LANG20 `CodeGenerator<IrProgram, String>` adapter.
//!
//! This module wraps `IrToIntel4004Compiler` in the `CodeGenerator` protocol
//! defined by `codegen-core`.  It is the Rust equivalent of the Python
//! `Intel4004CodeGenerator` adapter described in LANG20.
//!
//! ## Assembly type
//!
//! The "assembly" returned is a `String` — human-readable Intel 4004 assembly
//! text.  Each line is either a label definition (`LOOP_0:`) or a single
//! 4004 mnemonic (`LDM 5`, `XCH R2`, `JUN LOOP_0`, etc.).
//!
//! The text can be fed to the `intel-4004-assembler` crate to produce a binary
//! ROM image, or passed directly to the `intel4004-simulator` for execution.
//!
//! ## Usage
//!
//! ```rust
//! use codegen_core::codegen::CodeGenerator;
//! use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};
//! use ir_to_intel_4004_compiler::codegen::Intel4004CodeGenerator;
//!
//! let prog = IrProgram {
//!     instructions: vec![
//!         IrInstruction::new(IrOp::Label,   vec![IrOperand::Label("_start".into())], -1),
//!         IrInstruction::new(IrOp::LoadImm, vec![IrOperand::Register(0), IrOperand::Immediate(5)], 0),
//!         IrInstruction::new(IrOp::Halt,    vec![], 1),
//!     ],
//!     data: vec![],
//!     entry_label: "_start".into(),
//!     version: 1,
//! };
//!
//! let gen = Intel4004CodeGenerator::default();
//! assert!(gen.validate(&prog).is_empty());
//! let asm = gen.generate(&prog);
//! assert!(asm.contains("LDM"));   // LOAD_IMM lowers to LDM + XCH
//! ```

use codegen_core::codegen::CodeGenerator;
use compiler_ir::IrProgram;

use crate::IrToIntel4004Compiler;

// ── Intel4004CodeGenerator ────────────────────────────────────────────────────

/// `CodeGenerator<IrProgram, String>` adapter for the Intel 4004 backend.
///
/// Wraps `IrToIntel4004Compiler` in the `codegen_core::codegen::CodeGenerator`
/// protocol so it can be registered in a `CodeGeneratorRegistry` alongside the
/// WASM and JVM backends.
///
/// ## Assembly text format
///
/// The generated text starts with `    ORG 0x000` and then one or more lines
/// per IR instruction.  Labels are un-indented (`_start:`), instructions are
/// indented with four spaces.
#[derive(Debug, Clone, Default)]
pub struct Intel4004CodeGenerator;

impl CodeGenerator<IrProgram, String> for Intel4004CodeGenerator {
    /// Returns `"intel4004"` — the canonical backend identifier.
    fn name(&self) -> &str {
        "intel4004"
    }

    /// Validate `ir` for Intel 4004 assembly generation.
    ///
    /// Returns an empty `Vec` when every IR instruction in `ir` maps to a
    /// known Intel 4004 opcode sequence, or a `Vec` of error strings for any
    /// instruction that the backend cannot lower.
    fn validate(&self, ir: &IrProgram) -> Vec<String> {
        match IrToIntel4004Compiler::default().compile(ir) {
            Ok(_) => vec![],
            Err(errs) => errs.iter().map(|d| d.to_string()).collect(),
        }
    }

    /// Compile `IrProgram → Intel 4004 assembly text`.
    ///
    /// # Panics
    ///
    /// Panics if `validate(ir)` would have returned errors.  Always call
    /// `validate` first in production code.
    fn generate(&self, ir: &IrProgram) -> String {
        IrToIntel4004Compiler::default()
            .compile(ir)
            .expect("Intel4004CodeGenerator::generate called on invalid IrProgram; call validate() first")
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};

    use super::*;

    /// Minimal valid program: `_start: LOAD_IMM v0, 5; HALT`
    ///
    /// Intel 4004 supports LOAD_IMM for values 0–15 (fits in a nibble) and HALT
    /// via the BRU mnemonic pointing at itself.  This program exercises the
    /// simplest possible compile path.
    fn minimal_prog() -> IrProgram {
        IrProgram {
            instructions: vec![
                IrInstruction::new(
                    IrOp::Label,
                    vec![IrOperand::Label("_start".into())],
                    -1,
                ),
                IrInstruction::new(
                    IrOp::LoadImm,
                    vec![IrOperand::Register(0), IrOperand::Immediate(5)],
                    0,
                ),
                IrInstruction::new(IrOp::Halt, vec![], 1),
            ],
            data: vec![],
            entry_label: "_start".into(),
            version: 1,
        }
    }

    // Test 1: name() returns "intel4004"
    #[test]
    fn name_is_intel4004() {
        let gen = Intel4004CodeGenerator::default();
        assert_eq!(gen.name(), "intel4004");
    }

    // Test 2: validate() returns empty Vec on a valid program
    #[test]
    fn validate_empty_on_valid() {
        let gen = Intel4004CodeGenerator::default();
        let errors = gen.validate(&minimal_prog());
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    }

    // Test 3: validate() returns errors on a program with unsupported opcodes
    //         STORE_BYTE is not supported on the 4004 (no memory-write instruction)
    #[test]
    fn validate_fails_on_unsupported_opcode() {
        let gen = Intel4004CodeGenerator::default();
        let bad = IrProgram {
            instructions: vec![
                IrInstruction::new(
                    IrOp::Label,
                    vec![IrOperand::Label("_start".into())],
                    -1,
                ),
                IrInstruction::new(
                    IrOp::StoreByte,
                    vec![
                        IrOperand::Register(0),
                        IrOperand::Register(1),
                        IrOperand::Register(2),
                    ],
                    0,
                ),
            ],
            data: vec![],
            entry_label: "_start".into(),
            version: 1,
        };
        let errors = gen.validate(&bad);
        assert!(!errors.is_empty(), "expected errors for STORE_BYTE on 4004");
    }

    // Test 4: generate() returns non-empty assembly text
    #[test]
    fn generate_returns_text() {
        let gen = Intel4004CodeGenerator::default();
        let asm = gen.generate(&minimal_prog());
        assert!(!asm.is_empty(), "expected non-empty assembly text");
    }

    // Test 5: generated text starts with ORG directive
    #[test]
    fn generate_starts_with_org() {
        let gen = Intel4004CodeGenerator::default();
        let asm = gen.generate(&minimal_prog());
        assert!(asm.contains("ORG"), "expected ORG directive in assembly: {asm}");
    }

    // Test 6: LOAD_IMM lowers to LDM (load immediate data) mnemonic
    #[test]
    fn load_imm_lowers_to_ldm() {
        let gen = Intel4004CodeGenerator::default();
        let asm = gen.generate(&minimal_prog());
        assert!(asm.contains("LDM"), "expected LDM in assembly: {asm}");
    }

    // Test 7: validate() then generate() succeeds end-to-end
    #[test]
    fn validate_then_generate() {
        let gen = Intel4004CodeGenerator::default();
        let prog = minimal_prog();
        assert!(gen.validate(&prog).is_empty());
        let asm = gen.generate(&prog);
        assert!(asm.contains("_start"), "expected _start label in asm: {asm}");
    }

    // Test 8: Intel4004CodeGenerator is Send + Sync
    #[test]
    fn is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Intel4004CodeGenerator>();
    }
}
