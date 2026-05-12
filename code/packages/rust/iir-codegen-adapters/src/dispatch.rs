//! `compile_iir()` and `list_iir_backends()` — the primary consumer-facing API.
//!
//! Most callers do not need to interact with the `CodeGeneratorRegistry`
//! directly.  This module provides two simple free functions:
//!
//! - [`compile_iir`] — compile an `IIRModule` to any registered backend by
//!   name, returning a unified [`IIRBackendArtifact`].
//! - [`list_iir_backends`] — enumerate the stable names of all registered IIR
//!   backends (useful for help text or validation).
//!
//! ## Typical usage
//!
//! ```rust
//! use interpreter_ir::{IIRModule, IIRFunction, IIRInstr};
//! use iir_codegen_adapters::{compile_iir, list_iir_backends, IIRBackendArtifact};
//!
//! // Build a minimal module: one void function with a ret_void.
//! let fn_ = IIRFunction::new("main", vec![], "void",
//!     vec![IIRInstr::new("ret_void", None, vec![], "void")]);
//! let module = IIRModule {
//!     name: "demo".into(), functions: vec![fn_],
//!     entry_point: Some("main".into()), language: "test".into(),
//! };
//!
//! // Show available backends.
//! let backends = list_iir_backends();
//! assert!(backends.contains(&"iir-wasm"));
//!
//! // Compile to WASM.
//! let artifact = compile_iir(&module, "iir-wasm").unwrap();
//! assert!(artifact.as_wasm().is_some());
//! ```

use codegen_core::codegen::CodeGenerator;
use interpreter_ir::IIRModule;
use iir_to_beam::IIRBeamCodeGenerator;
use iir_to_wasm::IIRWasmCodeGenerator;
use iir_to_jvm_class_file::IIRJvmCodeGenerator;
use iir_to_cil_bytecode::IIRClrCodeGenerator;

use crate::artifact::IIRBackendArtifact;
use crate::error::IIRAdapterError;

// ---------------------------------------------------------------------------
// Known backend names (static table)
// ---------------------------------------------------------------------------
//
// These must stay in sync with what `build_iir_codegen_registry()` registers.
// The list is intentionally sorted alphabetically — `list_iir_backends()` can
// return it directly without an allocation.

const KNOWN_BACKENDS: &[&str] = &["iir-beam", "iir-clr", "iir-jvm", "iir-wasm"];

// ===========================================================================
// compile_iir
// ===========================================================================

/// Compile `module` with the named backend.
///
/// ## Steps
///
/// 1. Look up `backend` in the IIR codegen registry.
/// 2. Call `validate(module)` on the generator.
/// 3. If validation passes, call `generate(module)`.
/// 4. Wrap the result in the appropriate [`IIRBackendArtifact`] variant.
///
/// ## Errors
///
/// | Error | Condition |
/// |-------|-----------|
/// | [`IIRAdapterError::UnknownBackend`] | `backend` is not a registered name |
/// | [`IIRAdapterError::ValidationFailed`] | `validate(module)` returned errors |
/// | [`IIRAdapterError::LoweringFailed`] | lowering panicked or returned an error |
///
/// ## Example
///
/// ```rust
/// use interpreter_ir::{IIRModule, IIRFunction, IIRInstr};
/// use iir_codegen_adapters::{compile_iir, IIRBackendArtifact};
///
/// let fn_ = IIRFunction::new("main", vec![], "void",
///     vec![IIRInstr::new("ret_void", None, vec![], "void")]);
/// let module = IIRModule { name: "d".into(), functions: vec![fn_],
///     entry_point: Some("main".into()), language: "t".into() };
///
/// // All four backends accept a minimal void function.
/// for backend in ["iir-beam", "iir-wasm", "iir-jvm", "iir-clr"] {
///     assert!(compile_iir(&module, backend).is_ok(), "backend {backend} failed");
/// }
/// ```
pub fn compile_iir(
    module: &IIRModule,
    backend: &str,
) -> Result<IIRBackendArtifact, IIRAdapterError> {
    // ── Step 1: reject unknown backends without building the registry ────────
    //
    // We match against the static KNOWN_BACKENDS list first so we can give a
    // clean `UnknownBackend` error without the overhead of constructing all
    // four generators.
    if !KNOWN_BACKENDS.contains(&backend) {
        return Err(IIRAdapterError::UnknownBackend {
            requested: backend.to_string(),
            available: KNOWN_BACKENDS.iter().map(|s| s.to_string()).collect(),
        });
    }

    // ── Step 2-4: dispatch to the appropriate backend ────────────────────────
    //
    // We instantiate only the generator we need rather than building the full
    // registry — this avoids constructing three unused generators on every call.
    // The constant placeholder name "iir_module" is used for the config; see
    // the spec for discussion of why this is acceptable for compile_iir.
    match backend {
        "iir-beam" => {
            let gen = IIRBeamCodeGenerator::new("iir_module");
            let errors = gen.validate(module);
            if !errors.is_empty() {
                return Err(IIRAdapterError::ValidationFailed {
                    backend: backend.to_string(),
                    errors,
                });
            }
            // generate() panics on invalid input; we validated above.
            let artifact = gen.generate(module);
            Ok(IIRBackendArtifact::Beam(artifact))
        }

        "iir-wasm" => {
            let gen = IIRWasmCodeGenerator::new("iir_module");
            let errors = gen.validate(module);
            if !errors.is_empty() {
                return Err(IIRAdapterError::ValidationFailed {
                    backend: backend.to_string(),
                    errors,
                });
            }
            let artifact = gen.generate(module);
            Ok(IIRBackendArtifact::Wasm(artifact))
        }

        "iir-jvm" => {
            let gen = IIRJvmCodeGenerator::new("IIRModule");
            let errors = gen.validate(module);
            if !errors.is_empty() {
                return Err(IIRAdapterError::ValidationFailed {
                    backend: backend.to_string(),
                    errors,
                });
            }
            let artifact = gen.generate(module);
            Ok(IIRBackendArtifact::Jvm(artifact))
        }

        "iir-clr" => {
            let gen = IIRClrCodeGenerator::new("iir_module");
            let errors = gen.validate(module);
            if !errors.is_empty() {
                return Err(IIRAdapterError::ValidationFailed {
                    backend: backend.to_string(),
                    errors,
                });
            }
            let artifact = gen.generate(module);
            Ok(IIRBackendArtifact::Clr(artifact))
        }

        // This arm is unreachable because we checked KNOWN_BACKENDS above.
        // The Rust compiler still requires it for exhaustiveness.
        _ => unreachable!("backend {:?} passed KNOWN_BACKENDS check but has no match arm", backend),
    }
}

// ===========================================================================
// list_iir_backends
// ===========================================================================

/// Return the stable names of all registered IIR backends, sorted alphabetically.
///
/// Currently returns:
/// `["iir-beam", "iir-clr", "iir-jvm", "iir-wasm"]`
///
/// These names are the valid `backend` arguments to [`compile_iir`].
///
/// ## Example
///
/// ```rust
/// use iir_codegen_adapters::list_iir_backends;
///
/// let backends = list_iir_backends();
/// assert_eq!(backends.len(), 4);
/// assert!(backends.contains(&"iir-wasm"));
/// ```
pub fn list_iir_backends() -> Vec<&'static str> {
    KNOWN_BACKENDS.to_vec()
}

// ===========================================================================
// Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};

    // ── Fixture ──────────────────────────────────────────────────────────────

    fn minimal_module() -> IIRModule {
        let fn_ = IIRFunction::new(
            "main",
            vec![],
            "void",
            vec![IIRInstr::new("ret_void", None, vec![], "void")],
        );
        IIRModule {
            name: "test".into(),
            functions: vec![fn_],
            entry_point: Some("main".into()),
            language: "test".into(),
            exports: vec![],
            imports: vec![],
        }
    }

    fn add_module() -> IIRModule {
        // add(a: i32, b: i32) -> i32
        let fn_ = IIRFunction::new(
            "add",
            vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
            "i32",
            vec![
                IIRInstr::new(
                    "add",
                    Some("v0".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())],
                    "i32",
                ),
                IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
            ],
        );
        IIRModule {
            name: "calc".into(),
            functions: vec![fn_],
            entry_point: Some("add".into()),
            language: "test".into(),
            exports: vec![],
            imports: vec![],
        }
    }

    // ── list_iir_backends ────────────────────────────────────────────────────

    #[test]
    fn list_has_four_entries() {
        assert_eq!(list_iir_backends().len(), 4);
    }

    #[test]
    fn list_contains_all_four_names() {
        let b = list_iir_backends();
        assert!(b.contains(&"iir-beam"));
        assert!(b.contains(&"iir-wasm"));
        assert!(b.contains(&"iir-jvm"));
        assert!(b.contains(&"iir-clr"));
    }

    #[test]
    fn list_is_sorted() {
        let b = list_iir_backends();
        let mut sorted = b.clone();
        sorted.sort();
        assert_eq!(b, sorted);
    }

    // ── compile_iir: unknown backend ─────────────────────────────────────────

    #[test]
    fn unknown_backend_returns_error() {
        let m = minimal_module();
        let e = compile_iir(&m, "x86").unwrap_err();
        match e {
            IIRAdapterError::UnknownBackend { requested, available } => {
                assert_eq!(requested, "x86");
                assert_eq!(available.len(), 4);
            }
            other => panic!("expected UnknownBackend, got {:?}", other),
        }
    }

    #[test]
    fn empty_string_backend_returns_error() {
        let m = minimal_module();
        assert!(matches!(
            compile_iir(&m, ""),
            Err(IIRAdapterError::UnknownBackend { .. })
        ));
    }

    // ── compile_iir: validation failure ─────────────────────────────────────

    #[test]
    fn empty_module_fails_validation_on_all_backends() {
        let empty = IIRModule {
            name: "e".into(),
            functions: vec![],
            entry_point: None,
            language: "t".into(),
            exports: vec![],
            imports: vec![],
        };
        for backend in list_iir_backends() {
            let result = compile_iir(&empty, backend);
            assert!(
                matches!(result, Err(IIRAdapterError::ValidationFailed { .. })),
                "backend {:?} should fail on empty module",
                backend
            );
        }
    }

    // ── compile_iir: successful compilation ─────────────────────────────────

    #[test]
    fn compile_iir_beam_returns_beam_artifact() {
        let art = compile_iir(&minimal_module(), "iir-beam").unwrap();
        assert!(art.as_beam().is_some());
        assert_eq!(art.backend_name(), "iir-beam");
    }

    #[test]
    fn compile_iir_wasm_returns_wasm_artifact() {
        let art = compile_iir(&minimal_module(), "iir-wasm").unwrap();
        assert!(art.as_wasm().is_some());
        assert_eq!(art.backend_name(), "iir-wasm");
    }

    #[test]
    fn compile_iir_jvm_returns_jvm_artifact() {
        let art = compile_iir(&minimal_module(), "iir-jvm").unwrap();
        assert!(art.as_jvm().is_some());
        assert_eq!(art.backend_name(), "iir-jvm");
    }

    #[test]
    fn compile_iir_clr_returns_clr_artifact() {
        let art = compile_iir(&minimal_module(), "iir-clr").unwrap();
        assert!(art.as_clr().is_some());
        assert_eq!(art.backend_name(), "iir-clr");
    }

    // ── compile_iir: arithmetic round-trip ──────────────────────────────────

    #[test]
    fn add_module_compiles_to_all_four_backends() {
        let m = add_module();
        for backend in list_iir_backends() {
            let result = compile_iir(&m, backend);
            assert!(
                result.is_ok(),
                "add module failed on backend {:?}: {:?}",
                backend,
                result.unwrap_err()
            );
        }
    }

    // ── artifact accessor cross-checks ───────────────────────────────────────

    #[test]
    fn non_beam_artifact_as_beam_returns_none() {
        let art = compile_iir(&minimal_module(), "iir-wasm").unwrap();
        assert!(art.as_beam().is_none());
    }

    #[test]
    fn non_wasm_artifact_as_wasm_returns_none() {
        let art = compile_iir(&minimal_module(), "iir-clr").unwrap();
        assert!(art.as_wasm().is_none());
    }
}
