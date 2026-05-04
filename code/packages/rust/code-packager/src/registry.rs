//! Packager registry — dispatches artifacts to the right packager by format.
//!
//! The `PackagerRegistry` is the single entry point for packaging. Callers
//! create a `CodeArtifact`, call `PackagerRegistry::pack`, and receive the
//! binary bytes — without knowing which specific packager was invoked.
//!
//! ## Dispatch table
//!
//! ```text
//! binary_format  │  packager module  │  output file
//! ───────────────┼───────────────────┼─────────────────────────────────────
//! "elf64"        │  elf64::pack      │  Linux ELF64 executable (.elf)
//! "macho64"      │  macho64::pack    │  macOS Mach-O 64-bit executable (.macho)
//! "pe"           │  pe::pack         │  Windows PE32+ executable (.exe)
//! "wasm"         │  wasm::pack       │  WebAssembly module (.wasm)
//! "raw"          │  raw::pack        │  Raw binary blob (.bin)
//! "intel_hex"    │  intel_hex::pack  │  Intel HEX text file (.hex)
//! ```
//!
//! Any other `binary_format` value returns `PackagerError::UnsupportedTarget`.
//!
//! ## Design note
//!
//! The registry is a zero-size struct with only associated functions. There is
//! no state to maintain — the dispatch is purely a function of the artifact's
//! target.
//!
//! ## Quick start
//!
//! ```rust
//! use code_packager::{CodeArtifact, PackagerRegistry, Target};
//!
//! let art = CodeArtifact::new(
//!     vec![0x90, 0xC3], // NOP; RET
//!     0,
//!     Target::linux_x64(),
//! );
//! let bytes = PackagerRegistry::pack(&art).unwrap();
//! assert_eq!(&bytes[0..4], &[0x7F, b'E', b'L', b'F']);
//! ```

use crate::artifact::CodeArtifact;
use crate::errors::PackagerError;
use crate::target::Target;
use crate::{elf64, intel_hex, macho64, pe, raw, wasm};

/// Stateless packager dispatcher.
///
/// All methods are `pub fn` (not `pub fn self`); you never need to construct
/// an instance.
pub struct PackagerRegistry;

impl PackagerRegistry {
    /// Pack `artifact` using the packager that matches its `binary_format`.
    ///
    /// # Errors
    ///
    /// - `PackagerError::UnsupportedTarget` if no packager handles the format.
    /// - Any error returned by the specific packager (also `UnsupportedTarget`
    ///   or `WasmEncodeError`).
    pub fn pack(artifact: &CodeArtifact) -> Result<Vec<u8>, PackagerError> {
        match artifact.target.binary_format.as_str() {
            // Linux/BSD ELF64 executable.
            "elf64" => elf64::pack(artifact),
            // macOS Mach-O 64-bit executable.
            "macho64" => macho64::pack(artifact),
            // Windows PE32+ executable.
            "pe" => pe::pack(artifact),
            // WebAssembly 1.0 module.
            "wasm" => wasm::pack(artifact),
            // Flat binary blob (no container).
            "raw" => raw::pack(artifact),
            // Intel HEX ASCII text format.
            "intel_hex" => intel_hex::pack(artifact),
            // Unknown format.
            fmt => Err(PackagerError::UnsupportedTarget(format!(
                "no packager for binary_format={fmt:?}"
            ))),
        }
    }

    /// Return the conventional file extension for the given `target`.
    ///
    /// The extension includes the leading dot (e.g. `".elf"`, `".exe"`).
    /// For unknown formats, returns `".bin"` as a safe fallback.
    ///
    /// # Example
    ///
    /// ```rust
    /// use code_packager::{PackagerRegistry, Target};
    ///
    /// assert_eq!(PackagerRegistry::file_extension(&Target::linux_x64()),   ".elf");
    /// assert_eq!(PackagerRegistry::file_extension(&Target::windows_x64()), ".exe");
    /// assert_eq!(PackagerRegistry::file_extension(&Target::wasm()),        ".wasm");
    /// ```
    pub fn file_extension(target: &Target) -> &'static str {
        match target.binary_format.as_str() {
            "elf64" => ".elf",
            "macho64" => ".macho",
            "pe" => ".exe",
            "wasm" => ".wasm",
            "raw" => ".bin",
            "intel_hex" => ".hex",
            // Unknown formats get a safe generic extension.
            _ => ".bin",
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Test 1: dispatch to ELF64 — output starts with ELF magic
    #[test]
    fn dispatch_elf64() {
        let art = CodeArtifact::new(vec![0x90], 0, Target::linux_x64());
        let bytes = PackagerRegistry::pack(&art).unwrap();
        assert_eq!(&bytes[0..4], &[0x7F, b'E', b'L', b'F']);
    }

    // Test 2: dispatch to Mach-O — output starts with Mach-O magic
    #[test]
    fn dispatch_macho64() {
        let art = CodeArtifact::new(vec![0x1F, 0x20, 0x03, 0xD5], 0, Target::macos_arm64());
        let bytes = PackagerRegistry::pack(&art).unwrap();
        // 0xFEEDFACF in little-endian = [0xCF, 0xFA, 0xED, 0xFE]
        assert_eq!(&bytes[0..4], &[0xCF, 0xFA, 0xED, 0xFE]);
    }

    // Test 3: dispatch to PE — output starts with "MZ"
    #[test]
    fn dispatch_pe() {
        let art = CodeArtifact::new(vec![0xC3], 0, Target::windows_x64());
        let bytes = PackagerRegistry::pack(&art).unwrap();
        assert_eq!(&bytes[0..2], b"MZ");
    }

    // Test 4: dispatch to raw — output equals input
    #[test]
    fn dispatch_raw() {
        let code = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let art = CodeArtifact::new(code.clone(), 0, Target::raw("x86_64"));
        let bytes = PackagerRegistry::pack(&art).unwrap();
        assert_eq!(bytes, code);
    }

    // Test 5: dispatch to intel_hex — output is ASCII text
    #[test]
    fn dispatch_intel_hex() {
        let art = CodeArtifact::new(vec![0xFF], 0, Target::intel_4004());
        let bytes = PackagerRegistry::pack(&art).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains(':'));
        assert!(text.ends_with('\n'));
    }

    // Test 6: dispatch to WASM — output starts with WASM magic
    #[test]
    fn dispatch_wasm() {
        // Minimal valid function body: i32.const 0; end
        let art = CodeArtifact::new(vec![0x41, 0x00, 0x0B], 0, Target::wasm());
        let bytes = PackagerRegistry::pack(&art).unwrap();
        assert_eq!(&bytes[0..4], &[0x00, 0x61, 0x73, 0x6D]);
    }

    // Test 7: unknown format returns UnsupportedTarget error
    #[test]
    fn unknown_format_returns_error() {
        use crate::target::Target;
        let art = CodeArtifact::new(
            vec![0x00],
            0,
            Target { arch: "x86_64".into(), os: "haiku".into(), binary_format: "elf32".into() },
        );
        let err = PackagerRegistry::pack(&art).unwrap_err();
        assert!(
            matches!(err, PackagerError::UnsupportedTarget(_)),
            "expected UnsupportedTarget, got {err:?}"
        );
    }

    // Test 8: file_extension for each known format
    #[test]
    fn file_extension_all_formats() {
        assert_eq!(PackagerRegistry::file_extension(&Target::linux_x64()),   ".elf");
        assert_eq!(PackagerRegistry::file_extension(&Target::macos_arm64()), ".macho");
        assert_eq!(PackagerRegistry::file_extension(&Target::windows_x64()), ".exe");
        assert_eq!(PackagerRegistry::file_extension(&Target::wasm()),        ".wasm");
        assert_eq!(PackagerRegistry::file_extension(&Target::raw("avr")),    ".bin");
        assert_eq!(PackagerRegistry::file_extension(&Target::intel_4004()),  ".hex");
    }

    // Test 9: file_extension for unknown format returns ".bin"
    #[test]
    fn file_extension_unknown_returns_bin() {
        let t = Target {
            arch: "riscv64".into(),
            os: "none".into(),
            binary_format: "elf32".into(), // not in dispatch table
        };
        assert_eq!(PackagerRegistry::file_extension(&t), ".bin");
    }
}
