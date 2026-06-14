//! `embed_debug_info` — one-call convenience dispatcher.
//!
//! Reads the target platform from an artifact descriptor and dispatches to
//! either [`DwarfEmitter`] (ELF/Mach-O) or [`CodeViewEmitter`] (PE).
//!
//! # Example
//!
//! ```no_run
//! use native_debug_info::{embed_debug_info, ArtifactInfo};
//! use debug_sidecar::DebugSidecarWriter;
//! use std::collections::HashMap;
//!
//! let mut w = DebugSidecarWriter::new();
//! let sidecar = w.finish();
//!
//! let artifact = ArtifactInfo {
//!     target: "linux".to_string(),
//!     load_address: 0x400000,
//!     image_base: 0x140000000,
//!     symbol_table_u64: HashMap::new(),
//!     symbol_table_u32: HashMap::new(),
//!     code_size: 0,
//!     code_rva: 0x1000,
//!     code_section_index: 1,
//! };
//!
//! // Provide a full ELF binary here; a truncated buffer will cause
//! // an out-of-bounds error when the emitter reads the section header table.
//! let elf_bytes: Vec<u8> = std::fs::read("my_program").unwrap();
//! let result = embed_debug_info(&elf_bytes, &artifact, &sidecar);
//! assert!(result.is_ok());
//! ```

use std::collections::HashMap;

use debug_sidecar::DebugSidecarReader;

use crate::codeview::CodeViewEmitter;
use crate::dwarf::DwarfEmitter;

// ---------------------------------------------------------------------------
// Target sets
// ---------------------------------------------------------------------------

const ELF_TARGETS: &[&str] = &["linux", "elf", "freebsd", "wasm"];
const MACHO_TARGETS: &[&str] = &["macos", "darwin", "macho"];
const PE_TARGETS: &[&str] = &["windows", "win32", "pe"];

// ---------------------------------------------------------------------------
// ArtifactInfo
// ---------------------------------------------------------------------------

/// Descriptor that provides target platform metadata for debug embedding.
///
/// Passed to [`embed_debug_info`].  Both symbol table variants are present
/// so the struct covers both DWARF (u64 offsets) and CodeView (u32 offsets)
/// without separate generics.
pub struct ArtifactInfo {
    /// Target platform string, e.g. `"linux"`, `"macos"`, `"windows"`.
    pub target: String,
    /// Virtual address at which code is loaded (ELF / Mach-O).
    pub load_address: u64,
    /// PE image base address.
    pub image_base: u64,
    /// Function name → byte offset (u64) for DWARF emitter.
    pub symbol_table_u64: HashMap<String, u64>,
    /// Function name → byte offset (u32) for CodeView emitter.
    pub symbol_table_u32: HashMap<String, u32>,
    /// Total code size in bytes (DWARF emitter).
    pub code_size: u64,
    /// RVA of the `.text` section (CodeView emitter).
    pub code_rva: u32,
    /// 1-based section index for `.text` (CodeView emitter, default 1).
    pub code_section_index: u16,
}

// ---------------------------------------------------------------------------
// Public function
// ---------------------------------------------------------------------------

/// Embed native debug info into a packed binary.
///
/// Dispatches to:
/// - [`DwarfEmitter::embed_in_elf`] for `"linux"`, `"elf"`, `"freebsd"`, `"wasm"`
/// - [`DwarfEmitter::embed_in_macho`] for `"macos"`, `"darwin"`, `"macho"`
/// - [`CodeViewEmitter::embed_in_pe`] for `"windows"`, `"win32"`, `"pe"`
///
/// # Parameters
///
/// - `packed_bytes` — raw binary (ELF, Mach-O, or PE).
/// - `artifact` — metadata descriptor.
/// - `sidecar_bytes` — raw sidecar from `DebugSidecarWriter::finish()`.
///
/// # Errors
///
/// Returns `Err` if:
/// - `sidecar_bytes` is not a valid sidecar.
/// - `artifact.target` is not a recognised platform.
/// - The underlying emitter returns an error.
pub fn embed_debug_info(
    packed_bytes: &[u8],
    artifact: &ArtifactInfo,
    sidecar_bytes: &[u8],
) -> Result<Vec<u8>, String> {
    let reader = DebugSidecarReader::new(sidecar_bytes)
        .map_err(|e| e.to_string())?;

    let target = artifact.target.to_lowercase();

    if ELF_TARGETS.contains(&target.as_str()) {
        let emitter = DwarfEmitter::new(
            &reader,
            artifact.load_address,
            &artifact.symbol_table_u64,
            artifact.code_size,
        );
        return emitter.embed_in_elf(packed_bytes);
    }

    if MACHO_TARGETS.contains(&target.as_str()) {
        let emitter = DwarfEmitter::new(
            &reader,
            artifact.load_address,
            &artifact.symbol_table_u64,
            artifact.code_size,
        );
        return emitter.embed_in_macho(packed_bytes);
    }

    if PE_TARGETS.contains(&target.as_str()) {
        let emitter = CodeViewEmitter::new(
            &reader,
            artifact.image_base,
            &artifact.symbol_table_u32,
            artifact.code_rva,
            artifact.code_section_index,
        );
        return emitter.embed_in_pe(packed_bytes);
    }

    Err(format!("unsupported target platform: {:?}", artifact.target))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use debug_sidecar::DebugSidecarWriter;

    fn empty_sidecar() -> Vec<u8> {
        DebugSidecarWriter::new().finish()
    }

    fn make_artifact(target: &str) -> ArtifactInfo {
        ArtifactInfo {
            target: target.to_string(),
            load_address: 0x400000,
            image_base: 0x140000000,
            symbol_table_u64: HashMap::new(),
            symbol_table_u32: HashMap::new(),
            code_size: 0,
            code_rva: 0x1000,
            code_section_index: 1,
        }
    }

    #[test]
    fn unknown_target_returns_error() {
        let artifact = make_artifact("amiga");
        let result = embed_debug_info(b"data", &artifact, &empty_sidecar());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("unsupported target platform"));
    }

    #[test]
    fn invalid_sidecar_returns_error() {
        let artifact = make_artifact("linux");
        let result = embed_debug_info(b"data", &artifact, b"not json");
        assert!(result.is_err());
    }

    #[test]
    fn linux_target_dispatches_to_elf() {
        let artifact = make_artifact("linux");
        // Real ELF data would succeed; invalid data triggers embed_in_elf error.
        let result = embed_debug_info(b"BADELF", &artifact, &empty_sidecar());
        assert!(result.is_err()); // "not a valid ELF" — correct path taken
        let err = result.unwrap_err();
        assert!(err.contains("ELF") || err.contains("elf"));
    }

    #[test]
    fn macos_target_dispatches_to_macho() {
        let artifact = make_artifact("macos");
        let result = embed_debug_info(b"BADMACHO", &artifact, &empty_sidecar());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Mach-O") || err.contains("mach"));
    }

    #[test]
    fn windows_target_dispatches_to_pe() {
        let artifact = make_artifact("windows");
        let result = embed_debug_info(b"BADPE", &artifact, &empty_sidecar());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("PE") || err.contains("pe") || err.contains("valid"));
    }

    #[test]
    fn target_case_insensitive_elf() {
        let artifact = make_artifact("LINUX");
        let result = embed_debug_info(b"x", &artifact, &empty_sidecar());
        // Should dispatch to ELF, not return "unsupported platform"
        assert!(result.is_err());
        assert!(!result.unwrap_err().contains("unsupported"));
    }

    #[test]
    fn darwin_alias_dispatches_to_macho() {
        let artifact = make_artifact("darwin");
        let result = embed_debug_info(b"x", &artifact, &empty_sidecar());
        assert!(result.is_err());
        assert!(!result.unwrap_err().contains("unsupported"));
    }

    #[test]
    fn pe_alias_dispatches_to_pe() {
        let artifact = make_artifact("pe");
        let result = embed_debug_info(b"x", &artifact, &empty_sidecar());
        assert!(result.is_err());
        assert!(!result.unwrap_err().contains("unsupported"));
    }
}
