//! ELF64 executable packager.
//!
//! Produces a minimal, single-load-segment ELF64 binary that the Linux kernel
//! can `execve()`. The output has no sections, no dynamic linking, and no debug
//! info — only the program headers the kernel needs to map and run the code.
//!
//! ## ELF64 binary layout
//!
//! ```text
//! Offset │ Size │ Content
//! ───────┼──────┼────────────────────────────────────────────────────────────
//!      0 │  64  │ ELF header (e_ident + fields)
//!     64 │  56  │ Program header #0 (PT_LOAD, the only segment)
//!    120 │   N  │ native_bytes (the machine code)
//! ```
//!
//! Total file size = 120 + len(native_bytes).
//!
//! ## ELF header breakdown (64 bytes)
//!
//! ```text
//! Offset │ Size │ Field          │ Value
//! ───────┼──────┼────────────────┼───────────────────────────────────────────
//!      0 │  4   │ e_ident magic  │ 0x7F 0x45 0x4C 0x46  ("\x7FELF")
//!      4 │  1   │ EI_CLASS       │ 2 = ELFCLASS64 (64-bit)
//!      5 │  1   │ EI_DATA        │ 1 = ELFDATA2LSB (little-endian)
//!      6 │  1   │ EI_VERSION     │ 1 = EV_CURRENT
//!      7 │  8   │ EI_OSABI + pad │ 0 = ELFOSABI_NONE
//!     15 │  1   │ padding        │ 0
//!     16 │  2   │ e_type         │ 2 = ET_EXEC (executable)
//!     18 │  2   │ e_machine      │ 62 = EM_X86_64  |  183 = EM_AARCH64
//!     20 │  4   │ e_version      │ 1 = EV_CURRENT
//!     24 │  8   │ e_entry        │ virtual address of first instruction
//!     32 │  8   │ e_phoff        │ 64 (program header offset)
//!     40 │  8   │ e_shoff        │ 0 (no section headers)
//!     48 │  4   │ e_flags        │ 0
//!     52 │  2   │ e_ehsize       │ 64 (size of this header)
//!     54 │  2   │ e_phentsize    │ 56 (size of one program header entry)
//!     56 │  2   │ e_phnum        │ 1 (one program header)
//!     58 │  2   │ e_shentsize    │ 64 (size of one section header, unused)
//!     60 │  2   │ e_shnum        │ 0 (no section headers)
//!     62 │  2   │ e_shstrndx     │ 0 (no section name table)
//! ```
//!
//! ## PT_LOAD program header breakdown (56 bytes, at offset 64)
//!
//! ```text
//! Offset │ Size │ Field   │ Value
//! ───────┼──────┼─────────┼──────────────────────────────────────────────────
//!     64 │  4   │ p_type  │ 1 = PT_LOAD (loadable segment)
//!     68 │  4   │ p_flags │ 5 = PF_R|PF_X (read + execute)
//!     72 │  8   │ p_offset│ 0 (segment starts at file offset 0)
//!     80 │  8   │ p_vaddr │ load_address (virtual address in memory)
//!     88 │  8   │ p_paddr │ load_address (physical address, same as vaddr)
//!     96 │  8   │ p_filesz│ 120 + len(code) (bytes in file)
//!    104 │  8   │ p_memsz │ 120 + len(code) (bytes in memory)
//!    112 │  8   │ p_align │ 0x200000 (2 MiB — standard page-aligned)
//! ```
//!
//! ## Machine type values
//!
//! ```text
//! Architecture │ e_machine value
//! ─────────────┼────────────────
//! x86_64       │ 62  (EM_X86_64)
//! arm64        │ 183 (EM_AARCH64)
//! ```

use crate::artifact::CodeArtifact;
use crate::errors::PackagerError;
use crate::target::Target;

// ELF header size = 64 bytes.  Program header size = 56 bytes.
const ELF_HEADER_SIZE: u64 = 64;
const PROGRAM_HEADER_SIZE: u64 = 56;
// The code segment begins immediately after both headers.
const HEADER_TOTAL: u64 = ELF_HEADER_SIZE + PROGRAM_HEADER_SIZE; // 120

// Machine type constants from the ELF specification.
const EM_X86_64: u16 = 62;
const EM_AARCH64: u16 = 183;

/// Determine the ELF `e_machine` value for the given target, or return an error.
fn machine_type(target: &Target) -> Result<u16, PackagerError> {
    match target.arch.as_str() {
        "x86_64" => Ok(EM_X86_64),
        "arm64" => Ok(EM_AARCH64),
        _ => Err(PackagerError::UnsupportedTarget(format!(
            "elf64 packager does not support arch={:?}",
            target.arch
        ))),
    }
}

/// Validate that the target can be packaged as ELF64.
///
/// Only Linux targets with `binary_format == "elf64"` are accepted.
fn validate(artifact: &CodeArtifact) -> Result<(), PackagerError> {
    if artifact.target.binary_format != "elf64" {
        return Err(PackagerError::UnsupportedTarget(format!(
            "elf64 packager does not handle binary_format={:?}",
            artifact.target.binary_format
        )));
    }
    if artifact.target.os != "linux" {
        return Err(PackagerError::UnsupportedTarget(format!(
            "elf64 packager expects os=linux, got {:?}",
            artifact.target.os
        )));
    }
    Ok(())
}

/// Pack `artifact` into an ELF64 executable binary.
///
/// Returns the raw bytes of the `.elf` file.  All multi-byte fields are
/// written in little-endian byte order via `.to_le_bytes()`.
///
/// # Errors
///
/// Returns `PackagerError::UnsupportedTarget` if the artifact's target is not
/// `linux_x64()` or `linux_arm64()`.
pub fn pack(artifact: &CodeArtifact) -> Result<Vec<u8>, PackagerError> {
    validate(artifact)?;
    let e_machine = machine_type(&artifact.target)?;

    // Read the load address from metadata or use the conventional Linux default.
    // 0x400000 is where Linux traditionally maps the first LOAD segment.
    // Reject negative values: a negative load_address would silently cast to
    // a huge u64 (two's-complement), embedding a nonsensical virtual address.
    let load_addr_i = artifact.metadata_int("load_address", 0x400000);
    if load_addr_i < 0 {
        return Err(PackagerError::UnsupportedTarget(format!(
            "elf64 packager: load_address must be non-negative, got {load_addr_i}"
        )));
    }
    let load_addr = load_addr_i as u64;

    let code_len = artifact.native_bytes.len() as u64;
    let file_size = HEADER_TOTAL + code_len; // p_filesz and p_memsz

    // The virtual address of the first instruction = load_addr + headers + entry_point.
    let entry_vaddr = load_addr + HEADER_TOTAL + artifact.entry_point as u64;

    let mut out: Vec<u8> = Vec::with_capacity(file_size as usize);

    // ── ELF header ────────────────────────────────────────────────────────────

    // e_ident[0..4]: ELF magic bytes.  Every ELF file starts with these four bytes.
    out.extend_from_slice(&[0x7F, b'E', b'L', b'F']);
    // e_ident[4]: EI_CLASS = 2 = ELFCLASS64  (64-bit ELF)
    out.push(2);
    // e_ident[5]: EI_DATA = 1 = ELFDATA2LSB  (little-endian byte order)
    out.push(1);
    // e_ident[6]: EI_VERSION = 1 = EV_CURRENT
    out.push(1);
    // e_ident[7..15]: EI_OSABI (0 = ELFOSABI_NONE = System V ABI) + 8 padding bytes
    out.extend_from_slice(&[0u8; 9]);

    // e_type = 2 = ET_EXEC (executable file, not shared library or relocatable).
    out.extend_from_slice(&2u16.to_le_bytes());
    // e_machine: which ISA — EM_X86_64 (62) or EM_AARCH64 (183).
    out.extend_from_slice(&e_machine.to_le_bytes());
    // e_version = 1 = EV_CURRENT (the only valid version).
    out.extend_from_slice(&1u32.to_le_bytes());
    // e_entry: the virtual address where the OS will jump to start the program.
    out.extend_from_slice(&entry_vaddr.to_le_bytes());
    // e_phoff: byte offset of the first program header = right after this header.
    out.extend_from_slice(&ELF_HEADER_SIZE.to_le_bytes());
    // e_shoff = 0: no section headers (minimal binary, kernel only needs program headers).
    out.extend_from_slice(&0u64.to_le_bytes());
    // e_flags = 0: no processor-specific flags required.
    out.extend_from_slice(&0u32.to_le_bytes());
    // e_ehsize = 64: this header is 64 bytes long.
    out.extend_from_slice(&64u16.to_le_bytes());
    // e_phentsize = 56: each program header entry is 56 bytes.
    out.extend_from_slice(&56u16.to_le_bytes());
    // e_phnum = 1: exactly one program header entry (the PT_LOAD segment).
    out.extend_from_slice(&1u16.to_le_bytes());
    // e_shentsize = 64: section header size (irrelevant since e_shnum = 0).
    out.extend_from_slice(&64u16.to_le_bytes());
    // e_shnum = 0: no section headers.
    out.extend_from_slice(&0u16.to_le_bytes());
    // e_shstrndx = 0: no section name string table.
    out.extend_from_slice(&0u16.to_le_bytes());

    // Sanity check: we must be at exactly offset 64 after the ELF header.
    debug_assert_eq!(out.len(), ELF_HEADER_SIZE as usize);

    // ── Program header #0 (PT_LOAD) ───────────────────────────────────────────

    // p_type = 1 = PT_LOAD: the kernel maps this segment into the process's address space.
    out.extend_from_slice(&1u32.to_le_bytes());
    // p_flags = 5 = PF_R | PF_X: read + execute (no write — code is immutable at runtime).
    //   PF_X = 1, PF_W = 2, PF_R = 4.
    out.extend_from_slice(&5u32.to_le_bytes());
    // p_offset = 0: this segment starts at the very beginning of the file.
    //   The kernel reads from file offset 0 and maps it to p_vaddr.
    out.extend_from_slice(&0u64.to_le_bytes());
    // p_vaddr: the virtual address where this segment is mapped.
    out.extend_from_slice(&load_addr.to_le_bytes());
    // p_paddr: physical address (relevant on embedded; Linux ignores it, uses p_vaddr).
    out.extend_from_slice(&load_addr.to_le_bytes());
    // p_filesz: number of bytes to read from the file.
    out.extend_from_slice(&file_size.to_le_bytes());
    // p_memsz: number of bytes to reserve in virtual memory (can exceed p_filesz for BSS).
    //   We keep them equal — no BSS segment.
    out.extend_from_slice(&file_size.to_le_bytes());
    // p_align = 0x200000 = 2 MiB: the segment's virtual address must be aligned to this.
    //   Linux uses huge-page-friendly 2 MiB alignment for the default loader.
    out.extend_from_slice(&0x200000u64.to_le_bytes());

    // Sanity check: we must be at exactly offset 120 after ELF + program headers.
    debug_assert_eq!(out.len(), HEADER_TOTAL as usize);

    // ── Machine code ──────────────────────────────────────────────────────────

    // The raw native bytes follow immediately after the headers.
    out.extend_from_slice(&artifact.native_bytes);

    Ok(out)
}

/// The conventional file extension for ELF files.
///
/// Note: Linux executables typically have no extension at all, but `.elf`
/// is used in the build system to distinguish packaged files.
pub fn file_extension() -> &'static str {
    ".elf"
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal x86-64 Linux artifact: NOP (0x90) at offset 0.
    fn linux_x64_art() -> CodeArtifact {
        CodeArtifact::new(vec![0x90], 0, Target::linux_x64())
    }

    // Test 1: ELF magic bytes appear at the start
    #[test]
    fn produces_elf_magic() {
        let bytes = pack(&linux_x64_art()).unwrap();
        // First 4 bytes must be 0x7F 'E' 'L' 'F'
        assert_eq!(&bytes[0..4], &[0x7F, b'E', b'L', b'F']);
    }

    // Test 2: correct file size
    #[test]
    fn correct_file_size() {
        let code = vec![0x90u8; 32]; // 32 NOP bytes
        let art = CodeArtifact::new(code, 0, Target::linux_x64());
        let bytes = pack(&art).unwrap();
        // 64 (ELF header) + 56 (program header) + 32 (code) = 152
        assert_eq!(bytes.len(), 152);
    }

    // Test 3: rejects macos_x64
    #[test]
    fn rejects_macos_target() {
        let art = CodeArtifact::new(vec![0x90], 0, Target::macos_x64());
        assert!(
            matches!(pack(&art), Err(PackagerError::UnsupportedTarget(_))),
            "expected UnsupportedTarget for macos_x64"
        );
    }

    // Test 4: rejects windows_x64
    #[test]
    fn rejects_windows_target() {
        let art = CodeArtifact::new(vec![0x90], 0, Target::windows_x64());
        assert!(matches!(pack(&art), Err(PackagerError::UnsupportedTarget(_))));
    }

    // Test 5: linux_arm64 produces EM_AARCH64 (183 = 0x00B7)
    #[test]
    fn arm64_machine_type() {
        let art = CodeArtifact::new(vec![0x00], 0, Target::linux_arm64());
        let bytes = pack(&art).unwrap();
        // e_machine is at offset 18, little-endian u16 = 183
        let e_machine = u16::from_le_bytes([bytes[18], bytes[19]]);
        assert_eq!(e_machine, 183, "expected EM_AARCH64 = 183, got {e_machine}");
    }

    // Test 6: x86_64 machine type = 62
    #[test]
    fn x86_64_machine_type() {
        let bytes = pack(&linux_x64_art()).unwrap();
        let e_machine = u16::from_le_bytes([bytes[18], bytes[19]]);
        assert_eq!(e_machine, 62, "expected EM_X86_64 = 62, got {e_machine}");
    }

    // Test 7: native_bytes appear at offset 120
    #[test]
    fn code_at_offset_120() {
        let code = vec![0x48, 0x31, 0xC0]; // xor rax, rax
        let art = CodeArtifact::new(code.clone(), 0, Target::linux_x64());
        let bytes = pack(&art).unwrap();
        assert_eq!(&bytes[120..123], code.as_slice());
    }

    // Test 8: entry point encoded correctly in e_entry
    #[test]
    fn entry_point_in_header() {
        let art = CodeArtifact::new(vec![0x90; 10], 4, Target::linux_x64());
        let bytes = pack(&art).unwrap();
        // e_entry is at offset 24, little-endian u64.
        // Expected = 0x400000 (load_addr) + 120 (header_total) + 4 (entry_point)
        let e_entry = u64::from_le_bytes(bytes[24..32].try_into().unwrap());
        assert_eq!(e_entry, 0x400000 + 120 + 4);
    }

    // Test 9: file_extension
    #[test]
    fn file_extension_is_elf() {
        assert_eq!(file_extension(), ".elf");
    }
}
