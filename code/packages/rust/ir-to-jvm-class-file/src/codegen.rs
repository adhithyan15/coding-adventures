//! `JvmCodeGenerator` — LANG20 `CodeGenerator<IrProgram, JvmClassArtifact>` adapter.
//!
//! This module wraps `lower_ir_to_jvm_class_file` in the `CodeGenerator` protocol
//! defined by `codegen-core`, making the JVM backend interchangeable with all
//! other LANG20 backends (WASM, Intel 4004, etc.).
//!
//! ## Assembly type
//!
//! The assembly returned is a `JvmClassArtifact` — a structured object that
//! carries the class name, the raw JVM `.class` bytes, callable label indices,
//! and data offsets.  Pass `artifact.class_bytes` to a JVM simulator for
//! execution, or call `write_class_file(&artifact, out_dir)` to write a `.class`
//! file to disk.
//!
//! ## Class name
//!
//! By default `JvmCodeGenerator` uses `"Main"` as the JVM class name.  Provide
//! a custom name at construction time:
//!
//! ```rust
//! use ir_to_jvm_class_file::codegen::JvmCodeGenerator;
//!
//! let gen = JvmCodeGenerator::new("MyClass");
//! assert_eq!(gen.class_name(), "MyClass");
//! ```
//!
//! ## Usage
//!
//! ```rust
//! use codegen_core::codegen::CodeGenerator;
//! use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};
//! use ir_to_jvm_class_file::codegen::JvmCodeGenerator;
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
//! let gen = JvmCodeGenerator::default();
//! assert!(gen.validate(&prog).is_empty());
//! let artifact = gen.generate(&prog);
//! assert!(!artifact.class_bytes.is_empty());
//! ```

use codegen_core::codegen::CodeGenerator;
use compiler_ir::IrProgram;

use crate::{lower_ir_to_jvm_class_file, JvmBackendConfig, JvmClassArtifact};

// ── JvmCodeGenerator ─────────────────────────────────────────────────────────

/// `CodeGenerator<IrProgram, JvmClassArtifact>` adapter for the JVM backend.
///
/// Wraps `lower_ir_to_jvm_class_file` in the `codegen_core::codegen::CodeGenerator`
/// protocol.  The returned `JvmClassArtifact` contains the raw `.class` bytes
/// in `artifact.class_bytes`.
///
/// ## Default class name
///
/// When constructed via `JvmCodeGenerator::default()` the JVM class name is
/// `"Main"`.  Change it with `JvmCodeGenerator::new("YourClass")`.
pub struct JvmCodeGenerator {
    class_name: String,
}

impl JvmCodeGenerator {
    /// Construct a JVM code generator with an explicit class name.
    ///
    /// # Arguments
    ///
    /// * `class_name` — The JVM class name (e.g. `"Main"`, `"BrainfuckProgram"`).
    ///   Must be a valid JVM binary class name.
    pub fn new(class_name: impl Into<String>) -> Self {
        Self { class_name: class_name.into() }
    }

    /// Return the JVM class name this generator uses.
    pub fn class_name(&self) -> &str {
        &self.class_name
    }

    fn config(&self) -> JvmBackendConfig {
        JvmBackendConfig::new(&self.class_name)
    }
}

impl Default for JvmCodeGenerator {
    fn default() -> Self {
        Self::new("Main")
    }
}

impl CodeGenerator<IrProgram, JvmClassArtifact> for JvmCodeGenerator {
    /// Returns `"jvm"` — the canonical backend identifier.
    fn name(&self) -> &str {
        "jvm"
    }

    /// Validate `ir` for JVM lowering.
    ///
    /// Returns an empty `Vec` if the program can be lowered without errors, or a
    /// single-element `Vec` containing the backend error message when the program
    /// is invalid for this target.
    fn validate(&self, ir: &IrProgram) -> Vec<String> {
        match lower_ir_to_jvm_class_file(ir, self.config()) {
            Ok(_) => vec![],
            Err(e) => vec![e.to_string()],
        }
    }

    /// Compile `IrProgram → JvmClassArtifact`.
    ///
    /// # Panics
    ///
    /// Panics if `validate(ir)` would have returned errors.  Always call
    /// `validate` first in production code.
    fn generate(&self, ir: &IrProgram) -> JvmClassArtifact {
        lower_ir_to_jvm_class_file(ir, self.config())
            .expect("JvmCodeGenerator::generate called on invalid IrProgram; call validate() first")
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};

    use super::*;

    /// Minimal valid program: `_start: LOAD_IMM v0, 99; HALT`
    ///
    /// This exercises the entire JVM lowering pipeline from a single entry
    /// function through to raw `.class` bytes.
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
                    vec![IrOperand::Register(0), IrOperand::Immediate(99)],
                    0,
                ),
                IrInstruction::new(IrOp::Halt, vec![], 1),
            ],
            data: vec![],
            entry_label: "_start".into(),
            version: 1,
        }
    }

    // Test 1: name() returns "jvm"
    #[test]
    fn name_is_jvm() {
        let gen = JvmCodeGenerator::default();
        assert_eq!(gen.name(), "jvm");
    }

    // Test 2: default class name is "Main"
    #[test]
    fn default_class_name() {
        let gen = JvmCodeGenerator::default();
        assert_eq!(gen.class_name(), "Main");
    }

    // Test 3: custom class name is preserved
    #[test]
    fn custom_class_name() {
        let gen = JvmCodeGenerator::new("BrainfuckProgram");
        assert_eq!(gen.class_name(), "BrainfuckProgram");
    }

    // Test 4: validate() returns empty Vec on a valid program
    #[test]
    fn validate_empty_on_valid() {
        let gen = JvmCodeGenerator::default();
        let errors = gen.validate(&minimal_prog());
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    }

    // Test 5: generate() produces a non-empty class artifact
    #[test]
    fn generate_produces_artifact() {
        let gen = JvmCodeGenerator::default();
        let artifact = gen.generate(&minimal_prog());
        assert!(!artifact.class_bytes.is_empty(), "expected non-empty class bytes");
    }

    // Test 6: artifact class name matches the configured class name
    #[test]
    fn artifact_class_name_matches_config() {
        let gen = JvmCodeGenerator::new("MyApp");
        let artifact = gen.generate(&minimal_prog());
        assert_eq!(artifact.class_name, "MyApp");
    }

    // Test 7: validate() then generate() succeeds end-to-end
    #[test]
    fn validate_then_generate() {
        let gen = JvmCodeGenerator::default();
        let prog = minimal_prog();
        assert!(gen.validate(&prog).is_empty());
        let artifact = gen.generate(&prog);
        // JVM class magic number: 0xCAFEBABE at bytes 0-3
        assert_eq!(&artifact.class_bytes[..4], &[0xCA, 0xFE, 0xBA, 0xBE]);
    }

    // Test 8: JvmCodeGenerator is Send + Sync
    #[test]
    fn is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<JvmCodeGenerator>();
    }
}
