//! `build_iir_codegen_registry()` — pre-populate a `CodeGeneratorRegistry` with all four IIR backends.
//!
//! ## Why a factory function?
//!
//! `CodeGeneratorRegistry` stores generators as type-erased `Box<dyn Any + Send + Sync>`,
//! so callers that need maximum flexibility — e.g. a pipeline driver that accepts
//! a backend name at runtime and downcasts to the concrete generator type — can
//! retrieve generators by name and call `validate` + `generate` directly.
//!
//! Most callers, however, only want "compile this module to backend X" — for that
//! use case, see [`crate::compile_iir`] which wraps the registry and handles
//! downcasting internally.
//!
//! ## Registered names
//!
//! | Name        | Concrete generator type    | Output type         |
//! |-------------|---------------------------|---------------------|
//! | `"iir-beam"` | [`IIRBeamCodeGenerator`]  | `BEAMModule`        |
//! | `"iir-wasm"` | [`IIRWasmCodeGenerator`]  | `WasmModule`        |
//! | `"iir-jvm"`  | [`IIRJvmCodeGenerator`]   | `JvmClassFile`      |
//! | `"iir-clr"`  | [`IIRClrCodeGenerator`]   | `CILProgramArtifact`|
//!
//! ## Downcast example
//!
//! ```rust
//! use iir_codegen_adapters::build_iir_codegen_registry;
//! use iir_to_wasm::IIRWasmCodeGenerator;
//!
//! let reg = build_iir_codegen_registry();
//! let any = reg.get("iir-wasm").unwrap();
//! let gen = any.downcast_ref::<IIRWasmCodeGenerator>().unwrap();
//! assert_eq!(gen.name(), "iir-wasm");
//! ```

use codegen_core::codegen::CodeGeneratorRegistry;
use iir_to_beam::IIRBeamCodeGenerator;
use iir_to_wasm::IIRWasmCodeGenerator;
use iir_to_jvm_class_file::IIRJvmCodeGenerator;
use iir_to_cil_bytecode::IIRClrCodeGenerator;

// ===========================================================================
// build_iir_codegen_registry
// ===========================================================================

/// Return a `CodeGeneratorRegistry` pre-populated with the four IIR backends.
///
/// The generators are instantiated with the placeholder module name
/// `"iir_module"`.  If you need a specific output module name (which affects
/// the BEAM module name, the JVM `this_class_name`, the WASM custom-name
/// section, and the CIL assembly name), instantiate the generators directly
/// and register them under their names.
///
/// ## Example
///
/// ```rust
/// use iir_codegen_adapters::build_iir_codegen_registry;
///
/// let reg = build_iir_codegen_registry();
/// // Four backends are registered:
/// assert_eq!(reg.len(), 4);
/// let mut names = reg.names();
/// names.sort();
/// assert_eq!(names, vec!["iir-beam", "iir-clr", "iir-jvm", "iir-wasm"]);
/// ```
pub fn build_iir_codegen_registry() -> CodeGeneratorRegistry {
    // Use a fixed placeholder name.  The module name is cosmetic — it appears
    // in output artifacts (BEAM module atom, JVM class name, etc.) but does not
    // affect bytecode semantics for a registry-level instantiation.
    const PLACEHOLDER_NAME: &str = "iir_module";

    let mut reg = CodeGeneratorRegistry::new();

    // ── Register all four IIR backends ───────────────────────────────────────
    //
    // Each generator must implement `Any + Send + Sync` for type-erased
    // storage.  The Box is required by `CodeGeneratorRegistry::register`.

    reg.register(
        "iir-beam",
        Box::new(IIRBeamCodeGenerator::new(PLACEHOLDER_NAME)),
    );

    reg.register(
        "iir-wasm",
        Box::new(IIRWasmCodeGenerator::new(PLACEHOLDER_NAME)),
    );

    reg.register(
        "iir-jvm",
        Box::new(IIRJvmCodeGenerator::new(PLACEHOLDER_NAME)),
    );

    reg.register(
        "iir-clr",
        Box::new(IIRClrCodeGenerator::new(PLACEHOLDER_NAME)),
    );

    reg
}

// ===========================================================================
// Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // IIRClrCodeGenerator implements CodeGenerator as a trait impl (not inherent
    // methods), so the trait must be in scope to call .name(), .validate(),
    // and .generate() on it.
    use codegen_core::codegen::CodeGenerator;
    use iir_to_beam::IIRBeamCodeGenerator;
    use iir_to_wasm::IIRWasmCodeGenerator;
    use iir_to_jvm_class_file::IIRJvmCodeGenerator;
    use iir_to_cil_bytecode::IIRClrCodeGenerator;

    #[test]
    fn registry_has_four_backends() {
        let reg = build_iir_codegen_registry();
        assert_eq!(reg.len(), 4);
    }

    #[test]
    fn registry_names_sorted() {
        let reg = build_iir_codegen_registry();
        assert_eq!(
            reg.names(),
            vec!["iir-beam", "iir-clr", "iir-jvm", "iir-wasm"]
        );
    }

    #[test]
    fn beam_generator_retrievable() {
        let reg = build_iir_codegen_registry();
        let any = reg.get("iir-beam").expect("iir-beam should be registered");
        let gen = any
            .downcast_ref::<IIRBeamCodeGenerator>()
            .expect("should downcast to IIRBeamCodeGenerator");
        assert_eq!(gen.name(), "iir-beam");
    }

    #[test]
    fn wasm_generator_retrievable() {
        let reg = build_iir_codegen_registry();
        let any = reg.get("iir-wasm").expect("iir-wasm should be registered");
        let gen = any
            .downcast_ref::<IIRWasmCodeGenerator>()
            .expect("should downcast to IIRWasmCodeGenerator");
        assert_eq!(gen.name(), "iir-wasm");
    }

    #[test]
    fn jvm_generator_retrievable() {
        let reg = build_iir_codegen_registry();
        let any = reg.get("iir-jvm").expect("iir-jvm should be registered");
        let gen = any
            .downcast_ref::<IIRJvmCodeGenerator>()
            .expect("should downcast to IIRJvmCodeGenerator");
        assert_eq!(gen.name(), "iir-jvm");
    }

    #[test]
    fn clr_generator_retrievable() {
        let reg = build_iir_codegen_registry();
        let any = reg.get("iir-clr").expect("iir-clr should be registered");
        let gen = any
            .downcast_ref::<IIRClrCodeGenerator>()
            .expect("should downcast to IIRClrCodeGenerator");
        assert_eq!(gen.name(), "iir-clr");
    }

    #[test]
    fn unknown_backend_returns_none() {
        let reg = build_iir_codegen_registry();
        assert!(reg.get("x86").is_none());
        assert!(reg.get("llvm").is_none());
        assert!(reg.get("").is_none());
    }

    #[test]
    fn registry_is_not_empty() {
        let reg = build_iir_codegen_registry();
        assert!(!reg.is_empty());
    }
}
