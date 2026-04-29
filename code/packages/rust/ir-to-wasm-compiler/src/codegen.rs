//! `WasmCodeGenerator` — LANG20 `CodeGenerator<IrProgram, WasmModule>` adapter.
//!
//! This module wraps `IrToWasmCompiler` in the `CodeGenerator` protocol defined
//! by `codegen-core`.  It is the Rust equivalent of the Python `WASMCodeGenerator`
//! adapter described in LANG20.
//!
//! ## Responsibilities
//!
//! - **Validate**: Attempt a dry-run compilation; map any `WasmLoweringError` to
//!   a `Vec<String>` of human-readable errors.
//! - **Generate**: Compile `IrProgram → WasmModule` using the default
//!   `IrToWasmCompiler` with an empty function-signature list (letting the
//!   compiler infer signatures from `COMMENT` instructions in the IR).
//!
//! ## Usage
//!
//! ```rust
//! use codegen_core::codegen::CodeGenerator;
//! use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};
//! use ir_to_wasm_compiler::codegen::WasmCodeGenerator;
//!
//! let prog = IrProgram {
//!     instructions: vec![
//!         IrInstruction::new(IrOp::Label, vec![IrOperand::Label("_start".into())], -1),
//!         IrInstruction::new(IrOp::Halt,  vec![], 0),
//!     ],
//!     data: vec![],
//!     entry_label: "_start".into(),
//!     version: 1,
//! };
//!
//! let gen = WasmCodeGenerator::default();
//! assert!(gen.validate(&prog).is_empty());
//! let module = gen.generate(&prog);
//! // module is a valid WasmModule with an exported "_start" function.
//! ```

use codegen_core::codegen::CodeGenerator;
use compiler_ir::IrProgram;
use wasm_types::WasmModule;

use crate::IrToWasmCompiler;

// ── WasmCodeGenerator ────────────────────────────────────────────────────────

/// `CodeGenerator<IrProgram, WasmModule>` adapter for the WASM backend.
///
/// Wraps `IrToWasmCompiler` in the `codegen_core::codegen::CodeGenerator`
/// protocol so it can be registered in a `CodeGeneratorRegistry` and used
/// uniformly alongside other backends.
///
/// ## Assembly type
///
/// Returns a `WasmModule` — the structured WebAssembly 1.0 module object
/// produced by the lowering pass.  Pass the result to `encode_module` from
/// `wasm-module-encoder` to obtain a binary `Vec<u8>`.
///
/// ## Function signatures
///
/// The `generate()` / `validate()` methods call the compiler with an *empty*
/// function-signature list, relying on the WASM compiler's built-in heuristic
/// that infers signatures from `COMMENT` instructions in the IR.  If you need
/// to supply explicit signatures, use `IrToWasmCompiler::compile()` directly.
#[derive(Debug, Clone, Default)]
pub struct WasmCodeGenerator;

impl CodeGenerator<IrProgram, WasmModule> for WasmCodeGenerator {
    /// Returns `"wasm"` — the canonical backend identifier.
    fn name(&self) -> &str {
        "wasm"
    }

    /// Attempt a dry-run compilation.
    ///
    /// Returns an empty `Vec` when the program is valid for WASM lowering,
    /// or a single-element `Vec` containing the lowering error message
    /// when something goes wrong.
    fn validate(&self, ir: &IrProgram) -> Vec<String> {
        match IrToWasmCompiler::default().compile(ir, &[]) {
            Ok(_) => vec![],
            Err(e) => vec![e.to_string()],
        }
    }

    /// Compile `IrProgram → WasmModule`.
    ///
    /// # Panics
    ///
    /// Panics if `validate(ir)` would have returned errors.  Always call
    /// `validate` first in production code.
    fn generate(&self, ir: &IrProgram) -> WasmModule {
        IrToWasmCompiler::default()
            .compile(ir, &[])
            .expect("WasmCodeGenerator::generate called on invalid IrProgram; call validate() first")
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};

    use super::*;

    /// Helper: a minimal valid IrProgram with a single entry function.
    /// Emits:  `_start: LOAD_IMM r0, 42; HALT`
    ///
    /// This is the canonical smoke-test program used throughout the LANG20 test
    /// suite.  `LOAD_IMM v0, 42; HALT` is the simplest program that exercises
    /// the full compile path without requiring any WASI imports.
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
                    vec![IrOperand::Register(0), IrOperand::Immediate(42)],
                    0,
                ),
                IrInstruction::new(IrOp::Halt, vec![], 1),
            ],
            data: vec![],
            entry_label: "_start".into(),
            version: 1,
        }
    }

    // Test 1: name() returns "wasm"
    #[test]
    fn name_is_wasm() {
        let gen = WasmCodeGenerator::default();
        assert_eq!(gen.name(), "wasm");
    }

    // Test 2: validate() returns empty Vec on a valid program
    #[test]
    fn validate_empty_on_valid() {
        let gen = WasmCodeGenerator::default();
        let errors = gen.validate(&minimal_prog());
        assert!(
            errors.is_empty(),
            "expected no errors, got: {:?}",
            errors
        );
    }

    // Test 3: validate() returns non-empty Vec on an unsupported SYSCALL number.
    //         SYSCALL 99 is not a recognised WASI syscall, so the compiler rejects it.
    //         This ensures validate() surfaces lowering errors as strings.
    #[test]
    fn validate_fails_on_bad_syscall() {
        let gen = WasmCodeGenerator::default();
        let bad = IrProgram {
            instructions: vec![
                IrInstruction::new(
                    IrOp::Label,
                    vec![IrOperand::Label("_start".into())],
                    -1,
                ),
                IrInstruction::new(
                    IrOp::Syscall,
                    vec![IrOperand::Immediate(99)],
                    0,
                ),
            ],
            data: vec![],
            entry_label: "_start".into(),
            version: 1,
        };
        let errors = gen.validate(&bad);
        assert!(!errors.is_empty(), "expected validation errors for SYSCALL 99");
        assert!(
            errors[0].contains("SYSCALL"),
            "expected SYSCALL mention in error: {:?}",
            errors
        );
    }

    // Test 4: generate() returns a WasmModule for a valid program
    #[test]
    fn generate_produces_wasm_module() {
        let gen = WasmCodeGenerator::default();
        let module = gen.generate(&minimal_prog());
        // A compiled WASM module must have at least one function type and one export
        assert!(!module.exports.is_empty(), "expected at least one export");
    }

    // Test 5: generate() WasmModule exports the _start function
    #[test]
    fn generate_exports_start() {
        let gen = WasmCodeGenerator::default();
        let module = gen.generate(&minimal_prog());
        let export_names: Vec<&str> = module.exports.iter().map(|e| e.name.as_str()).collect();
        assert!(
            export_names.contains(&"_start"),
            "_start not found in exports: {:?}",
            export_names
        );
    }

    // Test 6: validate() then generate() on the same program succeeds end-to-end
    #[test]
    fn validate_then_generate() {
        let gen = WasmCodeGenerator::default();
        let prog = minimal_prog();
        let errors = gen.validate(&prog);
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let module = gen.generate(&prog);
        // A well-formed WASM module produced by the lowering has at least one type entry
        assert!(!module.types.is_empty(), "expected non-empty type section");
    }

    // Test 7: WasmCodeGenerator is Send + Sync (required by CodeGenerator bound)
    #[test]
    fn is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WasmCodeGenerator>();
    }

    // Test 8: validate() is idempotent — calling it twice gives the same result
    #[test]
    fn validate_idempotent() {
        let gen = WasmCodeGenerator::default();
        let prog = minimal_prog();
        let e1 = gen.validate(&prog);
        let e2 = gen.validate(&prog);
        assert_eq!(e1, e2);
    }
}
