//! Mach-O 64-bit executable packager.
//!
//! Produces a minimal, single-section Mach-O 64-bit binary that macOS can
//! execute directly. The output uses `LC_MAIN` (the modern load command
//! introduced in macOS 10.8 Mountain Lion) to specify the entry point.
//!
//! ## Mach-O file layout
//!
//! ```text
//! Offset │ Size │ Content
//! ───────┼──────┼────────────────────────────────────────────────────────────
//!      0 │  32  │ mach_header_64
//!     32 │  72  │ LC_SEGMENT_64 header (for the __TEXT segment)
//!    104 │  80  │ section_64 (for the __text section)
//!    184 │  24  │ LC_MAIN (entry point specification)
//!    208 │   N  │ native_bytes (the machine code)
//! ```
//!
//! Total header = 32 + 72 + 80 + 24 = **208 bytes** (`HEADER_TOTAL`).
//!
//! ## mach_header_64 (32 bytes, at offset 0)
//!
//! ```text
//! Field      │ Size │ Value
//! ───────────┼──────┼───────────────────────────────────────────────────────
//! magic      │  4   │ 0xCFFAEDFE (MH_MAGIC_64, reversed for LE)
//! cputype    │  4   │ 0x01000007 (x86_64) | 0x0100000C (arm64)
//! cpusubtype │  4   │ 3 (x86_64 all) | 0 (arm64 all)
//! filetype   │  4   │ 2 = MH_EXECUTE
//! ncmds      │  4   │ 2 (two load commands: LC_SEGMENT_64 + LC_MAIN)
//! sizeofcmds │  4   │ 256 (72 + 80 + 24 = 176 … but we compute dynamically)
//! flags      │  4   │ 0
//! reserved   │  4   │ 0 (only in 64-bit header)
//! ```
//!
//! ## LC_SEGMENT_64 (72 bytes, at offset 32)
//!
//! ```text
//! Field    │ Size │ Value
//! ─────────┼──────┼─────────────────────────────────────────────────────────
//! cmd      │  4   │ 0x19 = LC_SEGMENT_64
//! cmdsize  │  4   │ 152 (72 header + 80 section_64)
//! segname  │ 16   │ "__TEXT\0\0\0\0\0\0\0\0\0\0"
//! vmaddr   │  8   │ load_address (default 0x100000000)
//! vmsize   │  8   │ HEADER_TOTAL + len(code)
//! fileoff  │  8   │ 0 (segment starts at file beginning)
//! filesize │  8   │ HEADER_TOTAL + len(code)
//! maxprot  │  4   │ 7 (rwx — maximum allowed protection)
//! initprot │  4   │ 5 (r+x — initial protection at load time)
//! nsects   │  4   │ 1 (one section: __text)
//! flags    │  4   │ 0
//! ```
//!
//! ## section_64 (80 bytes, at offset 104)
//!
//! ```text
//! Field     │ Size │ Value
//! ──────────┼──────┼────────────────────────────────────────────────────────
//! sectname  │ 16   │ "__text\0\0\0\0\0\0\0\0\0\0"
//! segname   │ 16   │ "__TEXT\0\0\0\0\0\0\0\0\0\0"
//! addr      │  8   │ load_address + HEADER_TOTAL
//! size      │  8   │ len(code)
//! offset    │  4   │ HEADER_TOTAL (file offset of the code)
//! align     │  4   │ 4 (2^4 = 16-byte alignment)
//! reloff    │  4   │ 0 (no relocations)
//! nreloc    │  4   │ 0
//! flags     │  4   │ 0x80000400 (S_ATTR_PURE_INSTRUCTIONS | S_ATTR_SOME_INSTRUCTIONS)
//! reserved1 │  4   │ 0
//! reserved2 │  4   │ 0
//! reserved3 │  4   │ 0
//! ```
//!
//! ## LC_MAIN (24 bytes, at offset 184)
//!
//! LC_MAIN replaced LC_UNIXTHREAD in macOS 10.8 and is required by the
//! dynamic linker on newer macOS versions. It specifies the entry point as an
//! *offset from the start of the __TEXT segment*, not an absolute address.
//!
//! ```text
//! Field     │ Size │ Value
//! ──────────┼──────┼────────────────────────────────────────────────────────
//! cmd       │  4   │ 0x80000028 = LC_MAIN
//! cmdsize   │  4   │ 24
//! entryoff  │  8   │ HEADER_TOTAL + entry_point
//! stacksize │  8   │ 0 (use default stack)
//! ```

use crate::artifact::CodeArtifact;
use crate::errors::PackagerError;
use crate::target::Target;

// ── Layout constants ──────────────────────────────────────────────────────────

// mach_header_64 size.
const MACH_HEADER_SIZE: u64 = 32;
// LC_SEGMENT_64 header (without sections) = 72 bytes.
const LC_SEGMENT_SIZE: u64 = 72;
// section_64 struct = 80 bytes.
const SECTION_SIZE: u64 = 80;
// LC_MAIN = 24 bytes.
const LC_MAIN_SIZE: u64 = 24;

// cmdsize of the LC_SEGMENT_64 command = header + one section_64.
const LC_SEGMENT_CMDSIZE: u32 = (LC_SEGMENT_SIZE + SECTION_SIZE) as u32; // 152

// Total size of all load commands (LC_SEGMENT_64 with section + LC_MAIN).
const SIZEOFCMDS: u32 = LC_SEGMENT_CMDSIZE + LC_MAIN_SIZE as u32; // 176

// Total header size before native code begins.
const HEADER_TOTAL: u64 = MACH_HEADER_SIZE + LC_SEGMENT_SIZE + SECTION_SIZE + LC_MAIN_SIZE; // 208

// ── CPU type/subtype constants ────────────────────────────────────────────────

// CPU_TYPE_X86_64 = CPU_TYPE_I386 | CPU_ARCH_ABI64 = 7 | 0x01000000.
const CPU_TYPE_X86_64: u32 = 0x01000007;
// CPU_TYPE_ARM64 = CPU_TYPE_ARM | CPU_ARCH_ABI64 = 12 | 0x01000000.
const CPU_TYPE_ARM64: u32 = 0x0100000C;

// CPU_SUBTYPE_X86_ALL = 3.
const CPU_SUBTYPE_X86_ALL: u32 = 3;
// CPU_SUBTYPE_ARM_ALL = 0.
const CPU_SUBTYPE_ARM_ALL: u32 = 0;

// ── Load command identifiers ──────────────────────────────────────────────────

// LC_SEGMENT_64 = 0x19.
const LC_SEGMENT_64: u32 = 0x19;
// LC_MAIN = 0x80000028 (flagged with LC_REQ_DYLD = 0x80000000).
const LC_MAIN: u32 = 0x80000028;

// ── Section flags ─────────────────────────────────────────────────────────────

// S_ATTR_PURE_INSTRUCTIONS (0x80000000) | S_ATTR_SOME_INSTRUCTIONS (0x00000400).
const SECTION_FLAGS_CODE: u32 = 0x80000400;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Determine CPU type and subtype for the given target.
fn cpu_type(target: &Target) -> Result<(u32, u32), PackagerError> {
    match target.arch.as_str() {
        "x86_64" => Ok((CPU_TYPE_X86_64, CPU_SUBTYPE_X86_ALL)),
        "arm64" => Ok((CPU_TYPE_ARM64, CPU_SUBTYPE_ARM_ALL)),
        _ => Err(PackagerError::UnsupportedTarget(format!(
            "macho64 packager does not support arch={:?}",
            target.arch
        ))),
    }
}

/// Validate that the artifact is destined for a macOS Mach-O target.
fn validate(artifact: &CodeArtifact) -> Result<(), PackagerError> {
    if artifact.target.binary_format != "macho64" {
        return Err(PackagerError::UnsupportedTarget(format!(
            "macho64 packager does not handle binary_format={:?}",
            artifact.target.binary_format
        )));
    }
    if artifact.target.os != "macos" {
        return Err(PackagerError::UnsupportedTarget(format!(
            "macho64 packager expects os=macos, got {:?}",
            artifact.target.os
        )));
    }
    Ok(())
}

/// Write a fixed-length segment or section name field (always 16 bytes, NUL-padded).
fn write_name(out: &mut Vec<u8>, name: &[u8]) {
    // The Mach-O spec requires exactly 16 bytes for segname/sectname fields.
    let mut buf = [0u8; 16];
    let n = name.len().min(16);
    buf[..n].copy_from_slice(&name[..n]);
    out.extend_from_slice(&buf);
}

/// Pack `artifact` into a Mach-O 64-bit executable binary.
///
/// Returns the raw bytes of the `.macho` file.  All multi-byte fields are
/// written in little-endian byte order.
///
/// # Errors
///
/// Returns `PackagerError::UnsupportedTarget` if the artifact's target is not
/// `macos_x64()` or `macos_arm64()`.
pub fn pack(artifact: &CodeArtifact) -> Result<Vec<u8>, PackagerError> {
    validate(artifact)?;
    let (cputype, cpusubtype) = cpu_type(&artifact.target)?;

    // Default load address for macOS 64-bit = 0x100000000 (above 4 GiB boundary).
    // Reject negative values: a negative load_address casts to an enormous u64,
    // producing a nonsensical vmaddr in the LC_SEGMENT_64 header.
    let load_addr_i = artifact.metadata_int("load_address", 0x100000000);
    if load_addr_i < 0 {
        return Err(PackagerError::UnsupportedTarget(format!(
            "macho64 packager: load_address must be non-negative, got {load_addr_i}"
        )));
    }
    let load_addr = load_addr_i as u64;
    let code_len = artifact.native_bytes.len() as u64;
    let vmsize = HEADER_TOTAL + code_len;

    let mut out: Vec<u8> = Vec::with_capacity(vmsize as usize);

    // ── mach_header_64 ────────────────────────────────────────────────────────

    // magic = 0xFEEDFACF (MH_MAGIC_64).
    // In little-endian memory this is stored as [CF EA FE DE] = 0xCFFAEDFE.
    // We write the *value* 0xFEEDFACF; to_le_bytes() gives [CF FA ED FE].
    // Wait — let's be precise: MH_MAGIC_64 in the Mach-O spec is 0xFEEDFACF
    // (host-endian). On a little-endian host, the on-disk bytes are
    // [0xCF, 0xFA, 0xED, 0xFE], which is what a hex editor shows.
    out.extend_from_slice(&0xFEEDFACFu32.to_le_bytes());
    // cputype: CPU_TYPE_X86_64 or CPU_TYPE_ARM64.
    out.extend_from_slice(&cputype.to_le_bytes());
    // cpusubtype: CPU_SUBTYPE_X86_ALL or CPU_SUBTYPE_ARM_ALL.
    out.extend_from_slice(&cpusubtype.to_le_bytes());
    // filetype = 2 = MH_EXECUTE (executable binary).
    out.extend_from_slice(&2u32.to_le_bytes());
    // ncmds = 2: one LC_SEGMENT_64 command + one LC_MAIN command.
    out.extend_from_slice(&2u32.to_le_bytes());
    // sizeofcmds: total byte size of all load commands.
    out.extend_from_slice(&SIZEOFCMDS.to_le_bytes());
    // flags = 0: no special flags needed for a minimal binary.
    out.extend_from_slice(&0u32.to_le_bytes());
    // reserved (64-bit header only): must be 0.
    out.extend_from_slice(&0u32.to_le_bytes());

    debug_assert_eq!(out.len(), MACH_HEADER_SIZE as usize);

    // ── LC_SEGMENT_64 ─────────────────────────────────────────────────────────

    // cmd = LC_SEGMENT_64 = 0x19.
    out.extend_from_slice(&LC_SEGMENT_64.to_le_bytes());
    // cmdsize = 152: the size of this load command including all section_64 structs.
    out.extend_from_slice(&LC_SEGMENT_CMDSIZE.to_le_bytes());
    // segname: "__TEXT" padded to 16 bytes.
    write_name(&mut out, b"__TEXT");
    // vmaddr: virtual address of the segment's start.
    out.extend_from_slice(&load_addr.to_le_bytes());
    // vmsize: size of the segment in virtual memory.
    out.extend_from_slice(&vmsize.to_le_bytes());
    // fileoff = 0: segment contents start at byte 0 of the file.
    out.extend_from_slice(&0u64.to_le_bytes());
    // filesize: number of bytes from the file that back this segment.
    out.extend_from_slice(&vmsize.to_le_bytes());
    // maxprot = 7 = VM_PROT_READ | VM_PROT_WRITE | VM_PROT_EXECUTE (maximum allowed).
    out.extend_from_slice(&7u32.to_le_bytes());
    // initprot = 5 = VM_PROT_READ | VM_PROT_EXECUTE (initial protection at load).
    out.extend_from_slice(&5u32.to_le_bytes());
    // nsects = 1: this segment contains one section (__text).
    out.extend_from_slice(&1u32.to_le_bytes());
    // flags = 0: no special segment flags.
    out.extend_from_slice(&0u32.to_le_bytes());

    debug_assert_eq!(out.len(), (MACH_HEADER_SIZE + LC_SEGMENT_SIZE) as usize);

    // ── section_64 for __text ─────────────────────────────────────────────────

    // sectname: "__text" padded to 16 bytes.
    write_name(&mut out, b"__text");
    // segname: "__TEXT" padded to 16 bytes.
    write_name(&mut out, b"__TEXT");
    // addr: virtual address of the section = segment start + header size.
    let text_addr = load_addr + HEADER_TOTAL;
    out.extend_from_slice(&text_addr.to_le_bytes());
    // size: byte length of the section's content.
    out.extend_from_slice(&code_len.to_le_bytes());
    // offset: file offset of the section's content (right after all headers).
    out.extend_from_slice(&(HEADER_TOTAL as u32).to_le_bytes());
    // align = 4: the section is aligned to 2^4 = 16 bytes (standard code alignment).
    out.extend_from_slice(&4u32.to_le_bytes());
    // reloff = 0: no relocations.
    out.extend_from_slice(&0u32.to_le_bytes());
    // nreloc = 0: no relocation entries.
    out.extend_from_slice(&0u32.to_le_bytes());
    // flags = S_ATTR_PURE_INSTRUCTIONS | S_ATTR_SOME_INSTRUCTIONS.
    //   S_ATTR_PURE_INSTRUCTIONS (0x80000000): all bytes are machine instructions.
    //   S_ATTR_SOME_INSTRUCTIONS (0x00000400): contains at least one instruction.
    out.extend_from_slice(&SECTION_FLAGS_CODE.to_le_bytes());
    // reserved1 = 0 (unused for code sections).
    out.extend_from_slice(&0u32.to_le_bytes());
    // reserved2 = 0 (unused for code sections).
    out.extend_from_slice(&0u32.to_le_bytes());
    // reserved3 = 0 (only in 64-bit section_64 struct).
    out.extend_from_slice(&0u32.to_le_bytes());

    debug_assert_eq!(
        out.len(),
        (MACH_HEADER_SIZE + LC_SEGMENT_SIZE + SECTION_SIZE) as usize
    );

    // ── LC_MAIN ───────────────────────────────────────────────────────────────

    // cmd = LC_MAIN = 0x80000028.
    // The 0x80000000 bit marks it as "required for dynamic linking" (LC_REQ_DYLD).
    out.extend_from_slice(&LC_MAIN.to_le_bytes());
    // cmdsize = 24: this command is exactly 24 bytes.
    out.extend_from_slice(&24u32.to_le_bytes());
    // entryoff: byte offset from the beginning of the __TEXT segment to the
    // first instruction. The __TEXT segment starts at file offset 0, so
    // entryoff = HEADER_TOTAL + entry_point.
    let entryoff = HEADER_TOTAL + artifact.entry_point as u64;
    out.extend_from_slice(&entryoff.to_le_bytes());
    // stacksize = 0: use the system default stack size.
    out.extend_from_slice(&0u64.to_le_bytes());

    debug_assert_eq!(out.len(), HEADER_TOTAL as usize);

    // ── Machine code ──────────────────────────────────────────────────────────

    out.extend_from_slice(&artifact.native_bytes);

    Ok(out)
}

/// The conventional file extension for Mach-O files.
pub fn file_extension() -> &'static str {
    ".macho"
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn macos_arm64_art() -> CodeArtifact {
        CodeArtifact::new(vec![0x1F, 0x20, 0x03, 0xD5], 0, Target::macos_arm64())
    }

    // Test 1: Mach-O magic bytes (little-endian stored as [CF FA ED FE])
    #[test]
    fn produces_macho_magic() {
        let bytes = pack(&macos_arm64_art()).unwrap();
        // 0xFEEDFACF.to_le_bytes() = [0xCF, 0xFA, 0xED, 0xFE]
        assert_eq!(&bytes[0..4], &[0xCF, 0xFA, 0xED, 0xFE]);
    }

    // Test 2: rejects linux_x64
    #[test]
    fn rejects_linux_target() {
        let art = CodeArtifact::new(vec![0x90], 0, Target::linux_x64());
        assert!(matches!(pack(&art), Err(PackagerError::UnsupportedTarget(_))));
    }

    // Test 3: correct file size
    #[test]
    fn correct_file_size() {
        let code = vec![0x00u8; 16];
        let art = CodeArtifact::new(code, 0, Target::macos_arm64());
        let bytes = pack(&art).unwrap();
        // 208 header + 16 code = 224
        assert_eq!(bytes.len(), 224);
    }

    // Test 4: ncmds = 2 at offset 16
    #[test]
    fn ncmds_is_two() {
        let bytes = pack(&macos_arm64_art()).unwrap();
        let ncmds = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
        assert_eq!(ncmds, 2);
    }

    // Test 5: ARM64 CPU type at offset 4
    #[test]
    fn arm64_cpu_type() {
        let bytes = pack(&macos_arm64_art()).unwrap();
        let cputype = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        assert_eq!(cputype, CPU_TYPE_ARM64);
    }

    // Test 6: x86_64 CPU type
    #[test]
    fn x86_64_cpu_type() {
        let art = CodeArtifact::new(vec![0x90], 0, Target::macos_x64());
        let bytes = pack(&art).unwrap();
        let cputype = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        assert_eq!(cputype, CPU_TYPE_X86_64);
    }

    // Test 7: native bytes appear at offset 208
    #[test]
    fn code_at_header_total() {
        let code = vec![0xAA, 0xBB, 0xCC];
        let art = CodeArtifact::new(code.clone(), 0, Target::macos_arm64());
        let bytes = pack(&art).unwrap();
        assert_eq!(&bytes[208..211], code.as_slice());
    }

    // Test 8: file_extension
    #[test]
    fn file_extension_is_macho() {
        assert_eq!(file_extension(), ".macho");
    }
}
