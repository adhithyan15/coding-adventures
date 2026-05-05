//! Mach-O 64-bit relocatable **object file** writer.
//!
//! Where [`crate::macho64`] produces a fully-linked executable
//! (`MH_EXECUTE`), this module produces a relocatable object file
//! (`MH_OBJECT`) intended to be fed to Apple's system linker `ld`.
//!
//! ## Why two formats
//!
//! On macOS 15+ (Sequoia / Tahoe) the kernel attaches a "provenance" tag
//! to every file recording which process wrote it.  Binaries written by
//! the system linker (`/usr/bin/ld`, Apple-signed) inherit a trusted
//! provenance and run normally; binaries written by random user code
//! (such as our [`crate::macho64`] executable writer) are SIGKILL'd at
//! exec time by `AppleSystemPolicy` regardless of how well-formed they
//! are.
//!
//! The fix is to delegate the final-link step to `ld`.  Our backend
//! emits a Mach-O **object file** with one section (`__text`) and one
//! exported symbol (`_main`); `ld` then produces the executable —
//! handling dyld setup, code signing, and crucially writing the file
//! itself so the kernel grants it trusted provenance.
//!
//! ## Object-file layout
//!
//! ```text
//! Offset          │ Size │ Content
//! ────────────────┼──────┼──────────────────────────────────────────────
//!              0  │  32  │ mach_header_64
//!             32  │  72  │ LC_SEGMENT_64 (no segname) — header only
//!            104  │  80  │ section_64 __text/__TEXT
//!            184  │  24  │ LC_BUILD_VERSION (macOS, minos, sdk)
//!            208  │  24  │ LC_SYMTAB (symoff, nsyms, stroff, strsize)
//!            232  │   N  │ machine code (`__text` section)
//!         232+N   │  16  │ nlist_64 entry for `_main`
//!         248+N   │   M  │ string table: "\0_main\0"
//! ```

use crate::artifact::CodeArtifact;
use crate::errors::PackagerError;
use crate::target::Target;

// ── Constants (subset of `<mach-o/loader.h>` and `<mach-o/nlist.h>`) ─────────

const MH_MAGIC_64:   u32 = 0xFEEDFACF;
const MH_OBJECT:     u32 = 1;
const CPU_TYPE_ARM64:    u32 = 0x0100_000C;
const CPU_TYPE_X86_64:   u32 = 0x0100_0007;
const CPU_SUBTYPE_ARM_ALL: u32 = 0;
const CPU_SUBTYPE_X86_ALL: u32 = 3;

const LC_SEGMENT_64:    u32 = 0x19;
const LC_SYMTAB:        u32 = 0x02;
const LC_BUILD_VERSION: u32 = 0x32;

const PLATFORM_MACOS: u32 = 1;
/// 15.0 — Sequoia, current LTS-equivalent on Apple Silicon.
const MIN_OS_VERSION: u32 = 0x000F_0000;
/// 15.0 — match minos.
const SDK_VERSION:    u32 = 0x000F_0000;

const SECTION_FLAGS_CODE: u32 = 0x80000400; // S_ATTR_PURE_INSTRUCTIONS | S_ATTR_SOME_INSTRUCTIONS

// nlist_64 fields
const N_EXT:  u8 = 0x01;
const N_SECT: u8 = 0x0E;

// Sizes
const MACH_HEADER_SIZE: usize = 32;
const LC_SEGMENT_SIZE:  usize = 72;
const SECTION_SIZE:     usize = 80;
const LC_BUILD_VERSION_SIZE: usize = 24;
const LC_SYMTAB_SIZE:   usize = 24;
const NLIST_64_SIZE:    usize = 16;

const HEADER_TOTAL: usize = MACH_HEADER_SIZE
    + LC_SEGMENT_SIZE + SECTION_SIZE
    + LC_BUILD_VERSION_SIZE
    + LC_SYMTAB_SIZE; // = 232

// ── Helpers ──────────────────────────────────────────────────────────────────

fn cpu_type(target: &Target) -> Result<(u32, u32), PackagerError> {
    match target.arch.as_str() {
        "arm64"  => Ok((CPU_TYPE_ARM64,  CPU_SUBTYPE_ARM_ALL)),
        "x86_64" => Ok((CPU_TYPE_X86_64, CPU_SUBTYPE_X86_ALL)),
        _ => Err(PackagerError::UnsupportedTarget(format!(
            "macho_object: unsupported arch {:?}", target.arch
        ))),
    }
}

fn write_name16(out: &mut Vec<u8>, name: &[u8]) {
    let mut buf = [0u8; 16];
    let n = name.len().min(16);
    buf[..n].copy_from_slice(&name[..n]);
    out.extend_from_slice(&buf);
}

// ── Public API ───────────────────────────────────────────────────────────────

/// The symbol name `_main` that the linker treats as the program entry.
///
/// Apple convention: C-level `main` becomes `_main` in object files
/// (the leading underscore is the legacy "C decoration" for symbols
/// that originated from C source).  `ld -e _main` defaults to it.
pub const ENTRY_SYMBOL: &str = "_main";

/// Pack `artifact.native_bytes` as a Mach-O 64-bit relocatable object
/// file with a single exported symbol pointing at `entry_point`.
///
/// The object file is intended to be fed to Apple's system linker:
///
/// ```sh
/// ld -arch arm64 -platform_version macos 15.0 15.0 -e _main -o exe out.o
/// ```
///
/// `entry_point` is the byte offset within `native_bytes` of the entry
/// instruction.  For most cases this is `0` (entry is the first
/// emitted instruction).
pub fn pack_object(artifact: &CodeArtifact) -> Result<Vec<u8>, PackagerError> {
    if artifact.target.os != "macos" {
        return Err(PackagerError::UnsupportedTarget(format!(
            "macho_object expects os=macos, got {:?}", artifact.target.os
        )));
    }
    let (cputype, cpusubtype) = cpu_type(&artifact.target)?;
    let code_len = artifact.native_bytes.len() as u64;
    let entry_off = artifact.entry_point as u64;
    if entry_off > code_len {
        return Err(PackagerError::UnsupportedTarget(format!(
            "macho_object: entry_point {entry_off} exceeds code length {code_len}"
        )));
    }

    // String table layout: leading NUL + ENTRY_SYMBOL + NUL.
    // The leading NUL is the "no symbol name" sentinel; symbol n_strx=0
    // means "no name", so we always start at offset 1.
    let strtab: Vec<u8> = {
        let mut s = Vec::with_capacity(2 + ENTRY_SYMBOL.len());
        s.push(0); // sentinel
        s.extend_from_slice(ENTRY_SYMBOL.as_bytes());
        s.push(0);
        s
    };

    let symtab_off  = HEADER_TOTAL as u32 + code_len as u32;
    let strtab_off  = symtab_off + NLIST_64_SIZE as u32;

    let total_size: usize = HEADER_TOTAL
        + code_len as usize
        + NLIST_64_SIZE
        + strtab.len();

    let sizeofcmds: u32 = (LC_SEGMENT_SIZE + SECTION_SIZE
        + LC_BUILD_VERSION_SIZE + LC_SYMTAB_SIZE) as u32;

    let mut out: Vec<u8> = Vec::with_capacity(total_size);

    // ── mach_header_64 ──────────────────────────────────────────────────────
    out.extend_from_slice(&MH_MAGIC_64.to_le_bytes());
    out.extend_from_slice(&cputype.to_le_bytes());
    out.extend_from_slice(&cpusubtype.to_le_bytes());
    out.extend_from_slice(&MH_OBJECT.to_le_bytes());
    out.extend_from_slice(&3u32.to_le_bytes());        // ncmds
    out.extend_from_slice(&sizeofcmds.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());        // flags
    out.extend_from_slice(&0u32.to_le_bytes());        // reserved
    debug_assert_eq!(out.len(), MACH_HEADER_SIZE);

    // ── LC_SEGMENT_64 (object-file convention: empty segname) ───────────────
    out.extend_from_slice(&LC_SEGMENT_64.to_le_bytes());
    out.extend_from_slice(&((LC_SEGMENT_SIZE + SECTION_SIZE) as u32).to_le_bytes()); // cmdsize
    write_name16(&mut out, b"");                       // segname empty
    out.extend_from_slice(&0u64.to_le_bytes());        // vmaddr
    out.extend_from_slice(&code_len.to_le_bytes());    // vmsize
    out.extend_from_slice(&(HEADER_TOTAL as u64).to_le_bytes()); // fileoff
    out.extend_from_slice(&code_len.to_le_bytes());    // filesize
    out.extend_from_slice(&7u32.to_le_bytes());        // maxprot
    out.extend_from_slice(&7u32.to_le_bytes());        // initprot (object: rwx)
    out.extend_from_slice(&1u32.to_le_bytes());        // nsects
    out.extend_from_slice(&0u32.to_le_bytes());        // flags

    // ── section_64 __text in __TEXT ─────────────────────────────────────────
    write_name16(&mut out, b"__text");
    write_name16(&mut out, b"__TEXT");
    out.extend_from_slice(&0u64.to_le_bytes());        // addr (relocatable, 0)
    out.extend_from_slice(&code_len.to_le_bytes());    // size
    out.extend_from_slice(&(HEADER_TOTAL as u32).to_le_bytes()); // offset
    out.extend_from_slice(&4u32.to_le_bytes());        // align (2^4 = 16)
    out.extend_from_slice(&0u32.to_le_bytes());        // reloff
    out.extend_from_slice(&0u32.to_le_bytes());        // nreloc
    out.extend_from_slice(&SECTION_FLAGS_CODE.to_le_bytes()); // flags
    out.extend_from_slice(&0u32.to_le_bytes());        // reserved1
    out.extend_from_slice(&0u32.to_le_bytes());        // reserved2
    out.extend_from_slice(&0u32.to_le_bytes());        // reserved3

    debug_assert_eq!(out.len(), MACH_HEADER_SIZE + LC_SEGMENT_SIZE + SECTION_SIZE);

    // ── LC_BUILD_VERSION ────────────────────────────────────────────────────
    out.extend_from_slice(&LC_BUILD_VERSION.to_le_bytes());
    out.extend_from_slice(&(LC_BUILD_VERSION_SIZE as u32).to_le_bytes());
    out.extend_from_slice(&PLATFORM_MACOS.to_le_bytes());
    out.extend_from_slice(&MIN_OS_VERSION.to_le_bytes());
    out.extend_from_slice(&SDK_VERSION.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());        // ntools

    // ── LC_SYMTAB ───────────────────────────────────────────────────────────
    out.extend_from_slice(&LC_SYMTAB.to_le_bytes());
    out.extend_from_slice(&(LC_SYMTAB_SIZE as u32).to_le_bytes());
    out.extend_from_slice(&symtab_off.to_le_bytes());     // symoff
    out.extend_from_slice(&1u32.to_le_bytes());           // nsyms
    out.extend_from_slice(&strtab_off.to_le_bytes());     // stroff
    out.extend_from_slice(&(strtab.len() as u32).to_le_bytes()); // strsize

    debug_assert_eq!(out.len(), HEADER_TOTAL);

    // ── Machine code ────────────────────────────────────────────────────────
    out.extend_from_slice(&artifact.native_bytes);

    // ── Symbol table — one nlist_64 entry for `_main` ───────────────────────
    out.extend_from_slice(&1u32.to_le_bytes());        // n_strx (skip leading NUL)
    out.push(N_SECT | N_EXT);                          // n_type
    out.push(1);                                       // n_sect (1-based; first section)
    out.extend_from_slice(&0u16.to_le_bytes());        // n_desc
    out.extend_from_slice(&entry_off.to_le_bytes());   // n_value (offset within section)

    // ── String table ────────────────────────────────────────────────────────
    out.extend_from_slice(&strtab);

    debug_assert_eq!(out.len(), total_size);
    Ok(out)
}

/// Conventional file extension for object files written by [`pack_object`].
pub fn file_extension() -> &'static str { ".o" }

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn arm64_artifact(code: Vec<u8>) -> CodeArtifact {
        CodeArtifact::new(code, 0, Target::macos_arm64())
    }

    #[test]
    fn produces_macho_magic() {
        let bytes = pack_object(&arm64_artifact(vec![0x00; 4])).unwrap();
        assert_eq!(&bytes[0..4], &[0xCF, 0xFA, 0xED, 0xFE]);
    }

    #[test]
    fn filetype_is_mh_object() {
        let bytes = pack_object(&arm64_artifact(vec![0x00; 4])).unwrap();
        let filetype = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
        assert_eq!(filetype, MH_OBJECT);
    }

    #[test]
    fn ncmds_is_three() {
        let bytes = pack_object(&arm64_artifact(vec![0x00; 4])).unwrap();
        let ncmds = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
        assert_eq!(ncmds, 3);
    }

    #[test]
    fn rejects_non_macos_target() {
        let art = CodeArtifact::new(vec![0x00], 0, Target::linux_x64());
        assert!(matches!(pack_object(&art), Err(PackagerError::UnsupportedTarget(_))));
    }

    #[test]
    fn rejects_entry_past_end() {
        let mut art = arm64_artifact(vec![0x00; 4]);
        art.entry_point = 100;
        assert!(pack_object(&art).is_err());
    }

    #[test]
    fn output_size_matches_layout() {
        let code = vec![0x42u8; 12];
        let bytes = pack_object(&arm64_artifact(code.clone())).unwrap();
        // HEADER_TOTAL (232) + 12 code + 16 nlist + 7 strtab ("\0_main\0")
        assert_eq!(bytes.len(), 232 + 12 + 16 + 7);
    }

    #[test]
    fn x86_64_arch() {
        let art = CodeArtifact::new(vec![0x90; 4], 0, Target::macos_x64());
        let bytes = pack_object(&art).unwrap();
        let cputype = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        assert_eq!(cputype, CPU_TYPE_X86_64);
    }

    #[test]
    fn entry_symbol_value_matches_entry_point() {
        let mut art = arm64_artifact(vec![0x00; 16]);
        art.entry_point = 8;
        let bytes = pack_object(&art).unwrap();
        // nlist_64 starts at HEADER_TOTAL (232) + code_len (16) = 248.
        // n_value is the last 8 bytes of the 16-byte nlist record.
        let n_value_off = 232 + 16 + 8;
        let n_value = u64::from_le_bytes(bytes[n_value_off..n_value_off + 8].try_into().unwrap());
        assert_eq!(n_value, 8);
    }

    #[test]
    fn string_table_contains_main_symbol() {
        let bytes = pack_object(&arm64_artifact(vec![0x00; 4])).unwrap();
        // strtab is at the very end, length 7 ("\0_main\0").
        let strtab = &bytes[bytes.len() - 7 ..];
        assert_eq!(strtab, b"\0_main\0");
    }
}
