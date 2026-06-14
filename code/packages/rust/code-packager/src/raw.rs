//! Raw binary packager — pass-through with no container format.
//!
//! The "raw" packager is the simplest possible packager: it returns the native
//! bytes unmodified, with no ELF header, no Mach-O header, and no PE stub.
//!
//! ## Use cases
//!
//! Raw binaries are used in situations where there is no operating system to
//! parse a binary format:
//!
//! - **Bootloaders**: the BIOS/UEFI loads the first 512 bytes of a disk sector
//!   directly into memory and jumps to it. The bytes must start with
//!   executable code, not an OS-specific header.
//! - **Firmware images**: microcontrollers like the ATmega328 (Arduino Uno)
//!   have their flash memory programmed with a flat binary image.
//! - **ROM dumps**: retro computer emulators need the exact bytes that were
//!   in the original ROM chip.
//! - **Unit testing**: testing a code generator is easier with raw output
//!   because there are no headers to strip before comparing bytes.
//!
//! ## Accepted targets
//!
//! Any target with `binary_format == "raw"` is accepted, regardless of `arch`
//! or `os`. Use `Target::raw(arch)` to construct such a target.
//!
//! ```rust
//! use code_packager::{CodeArtifact, PackagerRegistry, Target};
//!
//! let art = CodeArtifact::new(vec![0x90, 0xC3], 0, Target::raw("x86_64"));
//! let bytes = PackagerRegistry::pack(&art).unwrap();
//! assert_eq!(bytes, vec![0x90, 0xC3]);
//! ```

use crate::artifact::CodeArtifact;
use crate::errors::PackagerError;

/// Pack `artifact` by returning its native bytes unchanged.
///
/// # Errors
///
/// Returns `PackagerError::UnsupportedTarget` if `binary_format != "raw"`.
pub fn pack(artifact: &CodeArtifact) -> Result<Vec<u8>, PackagerError> {
    if artifact.target.binary_format != "raw" {
        return Err(PackagerError::UnsupportedTarget(format!(
            "raw packager does not handle binary_format={:?}",
            artifact.target.binary_format
        )));
    }
    // Nothing to do — just clone the bytes.
    Ok(artifact.native_bytes.clone())
}

/// The conventional file extension for raw binaries.
pub fn file_extension() -> &'static str {
    ".bin"
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::Target;

    // Test 1: raw target returns bytes unchanged
    #[test]
    fn pass_through_bytes() {
        let code = vec![0x90, 0x48, 0x31, 0xC0, 0xC3];
        let art = CodeArtifact::new(code.clone(), 0, Target::raw("x86_64"));
        let out = pack(&art).unwrap();
        assert_eq!(out, code);
    }

    // Test 2: empty bytes round-trip
    #[test]
    fn empty_bytes() {
        let art = CodeArtifact::new(vec![], 0, Target::raw("avr"));
        let out = pack(&art).unwrap();
        assert!(out.is_empty());
    }

    // Test 3: rejects elf64 target
    #[test]
    fn rejects_elf64() {
        let art = CodeArtifact::new(vec![0x90], 0, Target::linux_x64());
        assert!(matches!(pack(&art), Err(PackagerError::UnsupportedTarget(_))));
    }

    // Test 4: rejects wasm target
    #[test]
    fn rejects_wasm() {
        let art = CodeArtifact::new(vec![0x0B], 0, Target::wasm());
        assert!(matches!(pack(&art), Err(PackagerError::UnsupportedTarget(_))));
    }

    // Test 5: output is a clone (not the same allocation)
    #[test]
    fn output_is_independent_clone() {
        let code = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let art = CodeArtifact::new(code.clone(), 0, Target::raw("mips"));
        let out = pack(&art).unwrap();
        assert_eq!(out, code);
    }

    // Test 6: file_extension
    #[test]
    fn file_extension_is_bin() {
        assert_eq!(file_extension(), ".bin");
    }
}
