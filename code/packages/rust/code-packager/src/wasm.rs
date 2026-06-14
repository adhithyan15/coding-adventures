//! WebAssembly module packager.
//!
//! Wraps native WebAssembly bytecode inside a minimal, well-formed WASM 1.0
//! module using the `wasm-types` and `wasm-module-encoder` crates.
//!
//! ## What is a WASM module?
//!
//! A WebAssembly module is a binary container format (`.wasm`) that bundles:
//!
//! - **Type definitions** — function signatures (param types → result types).
//! - **Function declarations** — references to type entries.
//! - **Exports** — which functions (and memories, tables, globals) are visible
//!   to the host (browser, Wasmtime, Node.js, etc.).
//! - **Code** — the actual function bodies (WASM bytecode).
//!
//! The packager creates the *minimal* module that makes the native bytes
//! callable from the host.
//!
//! ## Module structure produced
//!
//! ```text
//! WasmModule {
//!   types:     [FuncType { params: [], results: [i32] }]
//!   functions: [0]   ← function 0 has type 0
//!   exports:   [Export { name: "main", kind: Function, index: 0 }]
//!   code:      [FunctionBody { locals: [], code: <native_bytes> }]
//! }
//! ```
//!
//! The function signature `() -> i32` is a reasonable convention for a "main"
//! function: no arguments, returns an exit code.
//!
//! ## Export name
//!
//! The export name is taken from `artifact.metadata_list("exports")`.
//! If the list is empty or absent, the name defaults to `"main"`.
//!
//! ```text
//! metadata_list("exports") = []            → export name = "main"
//! metadata_list("exports") = ["run"]       → export name = "run"
//! metadata_list("exports") = ["add", "sub"] → export name = "add" (first)
//! ```
//!
//! ## WASM binary magic
//!
//! Every `.wasm` file starts with the 4-byte magic `\x00asm` (= `[0x00, 0x61, 0x73, 0x6D]`)
//! followed by the version `\x01\x00\x00\x00`. The `wasm-module-encoder` crate
//! always emits these correctly.
//!
//! ## Accepted targets
//!
//! Only `Target::wasm()` (binary_format = "wasm") is accepted.

use wasm_module_encoder::encode_module;
use wasm_types::{Export, ExternalKind, FuncType, FunctionBody, ValueType, WasmModule};

use crate::artifact::CodeArtifact;
use crate::errors::PackagerError;

/// Validate that the artifact targets the WASM format.
fn validate(artifact: &CodeArtifact) -> Result<(), PackagerError> {
    if artifact.target.binary_format != "wasm" {
        return Err(PackagerError::UnsupportedTarget(format!(
            "wasm packager does not handle binary_format={:?}",
            artifact.target.binary_format
        )));
    }
    Ok(())
}

/// Pack `artifact` into a minimal WASM 1.0 module.
///
/// The native bytes are placed verbatim into the function body's `code` field.
/// It is the caller's responsibility to ensure the bytes are valid WASM
/// bytecode (including the mandatory trailing `end` opcode 0x0B).
///
/// # Errors
///
/// Returns `PackagerError::UnsupportedTarget` if `binary_format != "wasm"`.
/// Returns `PackagerError::WasmEncodeError` if the WASM encoder fails.
pub fn pack(artifact: &CodeArtifact) -> Result<Vec<u8>, PackagerError> {
    validate(artifact)?;

    // ── Determine the export name ─────────────────────────────────────────────

    // Pull the first element of the "exports" metadata list, or fall back to "main".
    let exports_list = artifact.metadata_list("exports");
    let export_name = exports_list
        .first()
        .cloned()
        .unwrap_or_else(|| "main".to_string());

    // ── Build the WASM module ─────────────────────────────────────────────────

    let module = WasmModule {
        // Type section: define one function type: () -> i32.
        // This is the conventional signature for a "main" or "run" function
        // that returns an integer exit code (0 = success, non-zero = error).
        types: vec![FuncType {
            params: vec![],
            results: vec![ValueType::I32],
        }],

        // Function section: function 0 uses type 0.
        // The parallel array of (functions, code) means functions[0] is
        // implemented by code[0].
        functions: vec![0],

        // Export section: make function 0 visible to the host under export_name.
        exports: vec![Export {
            name: export_name,
            kind: ExternalKind::Function,
            index: 0,
        }],

        // Code section: function 0's body.
        // locals: no local variables beyond the parameters (there are none).
        // code: the raw WASM bytecode bytes from the artifact.
        code: vec![FunctionBody {
            locals: vec![],
            code: artifact.native_bytes.clone(),
        }],

        // All other sections are empty for this minimal module.
        ..WasmModule::default()
    };

    // ── Encode to bytes ───────────────────────────────────────────────────────

    // `encode_module` writes the WASM magic, version, and all sections.
    encode_module(&module)
        .map_err(|e| PackagerError::WasmEncodeError(e.to_string()))
}

/// The conventional file extension for WebAssembly modules.
pub fn file_extension() -> &'static str {
    ".wasm"
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::{CodeArtifact, MetadataValue};
    use crate::target::Target;
    use std::collections::HashMap;

    // Minimal WASM function body: `i32.const 42; end` = [0x41, 0x2A, 0x0B]
    fn minimal_wasm_code() -> Vec<u8> {
        vec![0x41, 0x2A, 0x0B] // i32.const 42; end
    }

    // Test 1: output starts with WASM magic [0x00, 0x61, 0x73, 0x6D]
    #[test]
    fn produces_wasm_magic() {
        let art = CodeArtifact::new(minimal_wasm_code(), 0, Target::wasm());
        let bytes = pack(&art).unwrap();
        assert_eq!(
            &bytes[0..4],
            &[0x00, 0x61, 0x73, 0x6D],
            "expected WASM magic \\x00asm, got {:02X?}", &bytes[0..4]
        );
    }

    // Test 2: WASM version = [0x01, 0x00, 0x00, 0x00]
    #[test]
    fn produces_wasm_version() {
        let art = CodeArtifact::new(minimal_wasm_code(), 0, Target::wasm());
        let bytes = pack(&art).unwrap();
        assert_eq!(&bytes[4..8], &[0x01, 0x00, 0x00, 0x00]);
    }

    // Test 3: rejects linux_x64
    #[test]
    fn rejects_linux_target() {
        let art = CodeArtifact::new(minimal_wasm_code(), 0, Target::linux_x64());
        assert!(matches!(pack(&art), Err(PackagerError::UnsupportedTarget(_))));
    }

    // Test 4: rejects pe target
    #[test]
    fn rejects_pe_target() {
        let art = CodeArtifact::new(minimal_wasm_code(), 0, Target::windows_x64());
        assert!(matches!(pack(&art), Err(PackagerError::UnsupportedTarget(_))));
    }

    // Test 5: export name defaults to "main" when no metadata
    #[test]
    fn default_export_name_is_main() {
        let art = CodeArtifact::new(minimal_wasm_code(), 0, Target::wasm());
        let bytes = pack(&art).unwrap();
        // "main" in UTF-8 is [0x6D, 0x61, 0x69, 0x6E].
        // It will appear somewhere in the binary (in the export section).
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("main"), "expected 'main' in WASM binary");
    }

    // Test 6: custom export name from metadata
    #[test]
    fn custom_export_name() {
        let mut m = HashMap::new();
        m.insert(
            "exports".into(),
            MetadataValue::List(vec!["run".into()]),
        );
        let art = CodeArtifact::new(minimal_wasm_code(), 0, Target::wasm())
            .with_metadata(m);
        let bytes = pack(&art).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("run"), "expected 'run' in WASM binary");
    }

    // Test 7: file_extension
    #[test]
    fn file_extension_is_wasm() {
        assert_eq!(file_extension(), ".wasm");
    }

    // Test 8: output is non-empty for empty native_bytes
    // (a valid WASM module with an empty function body is: [end] = [0x0B])
    #[test]
    fn empty_native_bytes_produces_module() {
        // A WASM function body must end with 0x0B (end opcode).
        let art = CodeArtifact::new(vec![0x0B], 0, Target::wasm());
        let result = pack(&art);
        // This should succeed and produce a non-empty byte slice.
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }
}
