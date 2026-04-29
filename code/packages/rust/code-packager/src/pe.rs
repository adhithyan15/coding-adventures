//! PE32+ executable packager for Windows x86-64.
//!
//! Produces a minimal Portable Executable (PE32+) file — the binary format
//! used by all Windows NT-family operating systems (Windows XP through 11,
//! Windows Server). The "32+" suffix means this is the 64-bit variant of the
//! PE format (magic = 0x020B), as opposed to PE32 (magic = 0x010B) for 32-bit.
//!
//! ## PE file layout
//!
//! ```text
//! Offset  │ Size │ Content
//! ────────┼──────┼─────────────────────────────────────────────────────────
//!       0 │  64  │ DOS stub (MZ header + minimal stub program)
//!      64 │   4  │ PE signature ("PE\x00\x00")
//!      68 │  20  │ COFF file header
//!      88 │ 240  │ Optional header (PE32+ variant)
//!     328 │  40  │ Section table (one .text entry)
//!     368 │ …    │ Padding to 0x200 (FileAlignment boundary)
//!    0x200 │   N  │ .text section (native_bytes + padding to FileAlignment)
//! ```
//!
//! ## DOS stub (64 bytes)
//!
//! The very start of every PE file must be an MS-DOS compatible header so
//! that old DOS programs print "This program cannot be run in DOS mode."
//! Modern Windows loaders look only at the `e_magic` ("MZ") and `e_lfanew`
//! fields and jump straight to the PE signature.
//!
//! ```text
//! Offset │ Field    │ Value
//! ───────┼──────────┼──────────────────────────────────────────────────────
//!      0 │ e_magic  │ 0x5A4D = "MZ" (Mark Zbikowski, the DOS header designer)
//!     60 │ e_lfanew │ 64 (byte offset of the PE signature)
//!   rest  │ padding  │ 0
//! ```
//!
//! ## COFF header (20 bytes, at offset 68)
//!
//! ```text
//! Field                │ Size │ Value
//! ─────────────────────┼──────┼──────────────────────────────────────────────
//! Machine              │  2   │ 0x8664 = IMAGE_FILE_MACHINE_AMD64
//! NumberOfSections     │  2   │ 1 (just .text)
//! TimeDateStamp        │  4   │ 0 (reproducible builds)
//! PointerToSymbolTable │  4   │ 0 (no COFF symbols)
//! NumberOfSymbols      │  4   │ 0
//! SizeOfOptionalHeader │  2   │ 240 (PE32+ optional header size)
//! Characteristics      │  2   │ 0x0022 = EXECUTABLE_IMAGE | NO_RELOCATIONS
//! ```
//!
//! ## Optional header PE32+ (240 bytes, at offset 88)
//!
//! Despite the name, this header is *mandatory* for executable images.
//! "Optional" refers to the fact that it's absent in object files (.obj).
//!
//! Key fields:
//! ```text
//! Field                      │ Value
//! ───────────────────────────┼──────────────────────────────────────────────
//! Magic                      │ 0x020B (PE32+, 64-bit)
//! SizeOfCode                 │ aligned(len(native_bytes), 0x200)
//! AddressOfEntryPoint        │ 0x1000 + entry_point (RVA)
//! BaseOfCode                 │ 0x1000 (first section starts at RVA 0x1000)
//! ImageBase                  │ 0x140000000 (default for 64-bit Windows EXEs)
//! SectionAlignment           │ 0x1000 (4 KiB page alignment in memory)
//! FileAlignment              │ 0x200 (512-byte alignment on disk)
//! SizeOfImage                │ aligned(0x1000 + len(native_bytes), 0x1000)
//! SizeOfHeaders              │ 0x200 (all headers fit in first 512-byte block)
//! Subsystem                  │ 3 = IMAGE_SUBSYSTEM_WINDOWS_CUI (console)
//! NumberOfRvaAndSizes        │ 16 (standard data directory count)
//! DataDirectory[16]          │ all zeros (no exports, imports, resources, etc.)
//! ```
//!
//! ## Section table (.text, 40 bytes)
//!
//! ```text
//! Field                │ Value
//! ─────────────────────┼───────────────────────────────────────────────────
//! Name                 │ ".text\0\0\0" (8 bytes, NUL-padded)
//! VirtualSize          │ len(native_bytes)
//! VirtualAddress       │ 0x1000 (RVA — relative virtual address)
//! SizeOfRawData        │ aligned(len(native_bytes), 0x200)
//! PointerToRawData     │ 0x200 (file offset of the .text section)
//! Characteristics      │ 0x60000020 = code | execute | read
//! ```

use crate::artifact::CodeArtifact;
use crate::errors::PackagerError;

// ── Layout constants ──────────────────────────────────────────────────────────

// The PE signature ("PE\x00\x00") is at the offset stored in e_lfanew.
const PE_HEADER_OFFSET: u32 = 64;

// SectionAlignment: sections are aligned to 4 KiB pages in virtual memory.
const SECTION_ALIGNMENT: u32 = 0x1000;

// FileAlignment: sections are aligned to 512-byte blocks on disk.
const FILE_ALIGNMENT: u32 = 0x200;

// All headers (DOS stub + PE sig + COFF + optional + section table) fit in 0x200.
const SIZE_OF_HEADERS: u32 = 0x200;

// The virtual address (RVA) where the .text section is mapped.
const TEXT_RVA: u32 = 0x1000;

// The .text section is at file offset 0x200 (first FileAlignment-aligned block
// after all headers).
const TEXT_FILE_OFFSET: u32 = 0x200;

// Default image base for PE32+ executables.
// The Windows loader prefers this address; ASLR may override it.
const IMAGE_BASE: u64 = 0x140000000;

// ── Helper ────────────────────────────────────────────────────────────────────

/// Round `x` up to the next multiple of `n`.
///
/// `n` must be a power of two.  Equivalent to `ceil(x / n) * n`.
///
/// ```text
/// align_to(3,  512) = 512
/// align_to(512, 512) = 512
/// align_to(513, 512) = 1024
/// ```
fn align_to(x: u32, n: u32) -> u32 {
    // Bit-twiddling trick: `!(n-1)` is a mask that zeros the low bits.
    // Adding `n-1` first ensures we round *up* rather than down.
    (x + n - 1) & !(n - 1)
}

/// Validate that the artifact targets Windows x86-64 PE format.
fn validate(artifact: &CodeArtifact) -> Result<(), PackagerError> {
    if artifact.target.binary_format != "pe" {
        return Err(PackagerError::UnsupportedTarget(format!(
            "pe packager does not handle binary_format={:?}",
            artifact.target.binary_format
        )));
    }
    if artifact.target.os != "windows" {
        return Err(PackagerError::UnsupportedTarget(format!(
            "pe packager expects os=windows, got {:?}",
            artifact.target.os
        )));
    }
    Ok(())
}

/// Pack `artifact` into a PE32+ executable binary.
///
/// Returns the raw bytes of the `.exe` file.  All fields are little-endian.
///
/// # Errors
///
/// Returns `PackagerError::UnsupportedTarget` if the artifact's target is not
/// `windows_x64()`.
pub fn pack(artifact: &CodeArtifact) -> Result<Vec<u8>, PackagerError> {
    validate(artifact)?;

    // Validate that native_bytes fits in a PE32+ file (max ~4 GiB, constrained by u32 RVA
    // and file-offset arithmetic).  Silently truncating via `as u32` would produce an
    // undersized output buffer that panics on the copy_from_slice below.
    let code_len = u32::try_from(artifact.native_bytes.len()).map_err(|_| {
        PackagerError::UnsupportedTarget(
            "pe packager: native_bytes length exceeds u32::MAX (4 GiB)".into(),
        )
    })?;
    // Validate that entry_point fits in u32.  On a 64-bit host, usize can exceed u32::MAX;
    // truncating would silently embed a wrong AddressOfEntryPoint in the header.
    let entry_u32 = u32::try_from(artifact.entry_point).map_err(|_| {
        PackagerError::UnsupportedTarget(
            "pe packager: entry_point exceeds u32::MAX".into(),
        )
    })?;
    // On-disk size of the code section, rounded up to the next file-alignment boundary.
    let raw_code_size = align_to(code_len, FILE_ALIGNMENT);
    // In-memory image size: headers occupy [0, 0x1000), code occupies [0x1000, ...).
    let size_of_image = align_to(
        TEXT_RVA.checked_add(code_len).ok_or_else(|| {
            PackagerError::UnsupportedTarget(
                "pe packager: TEXT_RVA + code_len overflows u32".into(),
            )
        })?,
        SECTION_ALIGNMENT,
    );
    // RVA of the entry point = start of .text + entry_point offset.
    let address_of_entry_point = TEXT_RVA.checked_add(entry_u32).ok_or_else(|| {
        PackagerError::UnsupportedTarget(
            "pe packager: TEXT_RVA + entry_point overflows u32".into(),
        )
    })?;

    // Pre-allocate the full output buffer.
    let total_size = (TEXT_FILE_OFFSET + raw_code_size) as usize;
    let mut out: Vec<u8> = vec![0u8; total_size];

    // ── DOS stub (64 bytes) ───────────────────────────────────────────────────

    // e_magic = "MZ" — the DOS magic number.
    out[0] = b'M';
    out[1] = b'Z';
    // e_lfanew at offset 60 — tells the loader where the PE signature is.
    out[60..64].copy_from_slice(&PE_HEADER_OFFSET.to_le_bytes());

    // ── PE signature (4 bytes, at offset 64) ──────────────────────────────────

    out[64] = b'P';
    out[65] = b'E';
    out[66] = 0x00;
    out[67] = 0x00;

    // ── COFF header (20 bytes, at offset 68) ──────────────────────────────────

    let mut pos = 68usize;

    // Machine = 0x8664 = IMAGE_FILE_MACHINE_AMD64 (x86-64).
    out[pos..pos + 2].copy_from_slice(&0x8664u16.to_le_bytes()); pos += 2;
    // NumberOfSections = 1 (just .text).
    out[pos..pos + 2].copy_from_slice(&1u16.to_le_bytes()); pos += 2;
    // TimeDateStamp = 0 for reproducible builds.
    out[pos..pos + 4].copy_from_slice(&0u32.to_le_bytes()); pos += 4;
    // PointerToSymbolTable = 0 (no COFF symbol table).
    out[pos..pos + 4].copy_from_slice(&0u32.to_le_bytes()); pos += 4;
    // NumberOfSymbols = 0.
    out[pos..pos + 4].copy_from_slice(&0u32.to_le_bytes()); pos += 4;
    // SizeOfOptionalHeader = 240 (fixed size for PE32+).
    out[pos..pos + 2].copy_from_slice(&240u16.to_le_bytes()); pos += 2;
    // Characteristics = 0x0022:
    //   IMAGE_FILE_EXECUTABLE_IMAGE (0x0002) — this is a runnable executable.
    //   IMAGE_FILE_RELOCS_STRIPPED  (0x0001) — no base relocations needed.
    out[pos..pos + 2].copy_from_slice(&0x0022u16.to_le_bytes()); pos += 2;

    // pos is now 88. ── Optional header (240 bytes) ───────────────────────────

    // Magic = 0x020B = PE32+ (64-bit image).
    out[pos..pos + 2].copy_from_slice(&0x020Bu16.to_le_bytes()); pos += 2;
    // MajorLinkerVersion = 0.
    out[pos] = 0; pos += 1;
    // MinorLinkerVersion = 0.
    out[pos] = 0; pos += 1;
    // SizeOfCode: total size of all code sections on disk (aligned).
    out[pos..pos + 4].copy_from_slice(&raw_code_size.to_le_bytes()); pos += 4;
    // SizeOfInitializedData = 0 (no .data section).
    out[pos..pos + 4].copy_from_slice(&0u32.to_le_bytes()); pos += 4;
    // SizeOfUninitializedData = 0 (no .bss section).
    out[pos..pos + 4].copy_from_slice(&0u32.to_le_bytes()); pos += 4;
    // AddressOfEntryPoint: RVA of the first instruction.
    out[pos..pos + 4].copy_from_slice(&address_of_entry_point.to_le_bytes()); pos += 4;
    // BaseOfCode: RVA where the code sections start (0x1000).
    out[pos..pos + 4].copy_from_slice(&TEXT_RVA.to_le_bytes()); pos += 4;
    // ImageBase: preferred load address in virtual memory.
    out[pos..pos + 8].copy_from_slice(&IMAGE_BASE.to_le_bytes()); pos += 8;
    // SectionAlignment: sections align to 4 KiB in virtual memory.
    out[pos..pos + 4].copy_from_slice(&SECTION_ALIGNMENT.to_le_bytes()); pos += 4;
    // FileAlignment: sections align to 512 bytes on disk.
    out[pos..pos + 4].copy_from_slice(&FILE_ALIGNMENT.to_le_bytes()); pos += 4;
    // MajorOperatingSystemVersion = 6 (Windows Vista/2008 baseline).
    out[pos..pos + 2].copy_from_slice(&6u16.to_le_bytes()); pos += 2;
    // MinorOperatingSystemVersion = 0.
    out[pos..pos + 2].copy_from_slice(&0u16.to_le_bytes()); pos += 2;
    // MajorImageVersion = 0.
    out[pos..pos + 2].copy_from_slice(&0u16.to_le_bytes()); pos += 2;
    // MinorImageVersion = 0.
    out[pos..pos + 2].copy_from_slice(&0u16.to_le_bytes()); pos += 2;
    // MajorSubsystemVersion = 6 (Windows Vista minimum).
    out[pos..pos + 2].copy_from_slice(&6u16.to_le_bytes()); pos += 2;
    // MinorSubsystemVersion = 0.
    out[pos..pos + 2].copy_from_slice(&0u16.to_le_bytes()); pos += 2;
    // Win32VersionValue = 0 (must be zero per spec).
    out[pos..pos + 4].copy_from_slice(&0u32.to_le_bytes()); pos += 4;
    // SizeOfImage: total size of the image in memory, page-aligned.
    out[pos..pos + 4].copy_from_slice(&size_of_image.to_le_bytes()); pos += 4;
    // SizeOfHeaders: size of all headers on disk, aligned to FileAlignment.
    out[pos..pos + 4].copy_from_slice(&SIZE_OF_HEADERS.to_le_bytes()); pos += 4;
    // CheckSum = 0 (loader verifies only for kernel-mode drivers; ignored for EXE).
    out[pos..pos + 4].copy_from_slice(&0u32.to_le_bytes()); pos += 4;
    // Subsystem = 3 = IMAGE_SUBSYSTEM_WINDOWS_CUI (console application).
    //   1 = native (no subsystem), 2 = Windows GUI, 3 = Windows CUI (console),
    //   9 = WinCE GUI, 14 = EFI application, etc.
    out[pos..pos + 2].copy_from_slice(&3u16.to_le_bytes()); pos += 2;
    // DllCharacteristics = 0 (no ASLR, no DEP, etc. for minimal binary).
    out[pos..pos + 2].copy_from_slice(&0u16.to_le_bytes()); pos += 2;
    // SizeOfStackReserve: amount of virtual address space reserved for the stack.
    out[pos..pos + 8].copy_from_slice(&0x100000u64.to_le_bytes()); pos += 8;
    // SizeOfStackCommit: initial committed (physically backed) stack size.
    out[pos..pos + 8].copy_from_slice(&0x1000u64.to_le_bytes()); pos += 8;
    // SizeOfHeapReserve: reserved heap space (managed by process heap).
    out[pos..pos + 8].copy_from_slice(&0x100000u64.to_le_bytes()); pos += 8;
    // SizeOfHeapCommit: initial committed heap size.
    out[pos..pos + 8].copy_from_slice(&0x1000u64.to_le_bytes()); pos += 8;
    // LoaderFlags = 0 (obsolete field, must be zero).
    out[pos..pos + 4].copy_from_slice(&0u32.to_le_bytes()); pos += 4;
    // NumberOfRvaAndSizes = 16: the loader will parse 16 data directory entries.
    out[pos..pos + 4].copy_from_slice(&16u32.to_le_bytes()); pos += 4;
    // DataDirectory[16]: 16 entries × 8 bytes each = 128 bytes, all zero.
    // (No exports, imports, resources, exceptions, security, base relocs, etc.)
    for _ in 0..16 {
        out[pos..pos + 8].copy_from_slice(&0u64.to_le_bytes());
        pos += 8;
    }

    // pos should now be 88 + 240 = 328. ── Section table ───────────────────────

    // Section name: ".text\0\0\0" — exactly 8 bytes.
    out[pos..pos + 8].copy_from_slice(b".text\x00\x00\x00"); pos += 8;
    // VirtualSize: the actual byte count of the section's content (unpadded).
    out[pos..pos + 4].copy_from_slice(&code_len.to_le_bytes()); pos += 4;
    // VirtualAddress: RVA where the section is mapped in memory.
    out[pos..pos + 4].copy_from_slice(&TEXT_RVA.to_le_bytes()); pos += 4;
    // SizeOfRawData: disk size, rounded up to FileAlignment.
    out[pos..pos + 4].copy_from_slice(&raw_code_size.to_le_bytes()); pos += 4;
    // PointerToRawData: file offset of the section's raw bytes.
    out[pos..pos + 4].copy_from_slice(&TEXT_FILE_OFFSET.to_le_bytes()); pos += 4;
    // PointerToRelocations = 0 (no COFF relocations).
    out[pos..pos + 4].copy_from_slice(&0u32.to_le_bytes()); pos += 4;
    // PointerToLinenumbers = 0 (deprecated).
    out[pos..pos + 4].copy_from_slice(&0u32.to_le_bytes()); pos += 4;
    // NumberOfRelocations = 0.
    out[pos..pos + 2].copy_from_slice(&0u16.to_le_bytes()); pos += 2;
    // NumberOfLinenumbers = 0.
    out[pos..pos + 2].copy_from_slice(&0u16.to_le_bytes()); pos += 2;
    // Characteristics = 0x60000020:
    //   IMAGE_SCN_CNT_CODE        (0x00000020) — section contains executable code.
    //   IMAGE_SCN_MEM_EXECUTE     (0x20000000) — section is executable.
    //   IMAGE_SCN_MEM_READ        (0x40000000) — section is readable.
    out[pos..pos + 4].copy_from_slice(&0x60000020u32.to_le_bytes());
    pos += 4;

    // The section table ends at 328 + 40 = 368.
    // The output buffer was pre-filled with zeros, so the gap from 368 to 0x200
    // is already zero-padded — no extra work needed.

    // ── .text section (at file offset 0x200) ─────────────────────────────────

    // Copy native bytes. Any remaining bytes up to raw_code_size are already 0.
    let text_start = TEXT_FILE_OFFSET as usize;
    out[text_start..text_start + artifact.native_bytes.len()]
        .copy_from_slice(&artifact.native_bytes);

    let _ = pos; // suppress unused-variable warning
    Ok(out)
}

/// The conventional file extension for PE executables.
pub fn file_extension() -> &'static str {
    ".exe"
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::Target;

    fn win64_art() -> CodeArtifact {
        CodeArtifact::new(vec![0xC3], 0, Target::windows_x64()) // single RET instruction
    }

    // Test 1: MZ magic at offset 0
    #[test]
    fn produces_mz_magic() {
        let bytes = pack(&win64_art()).unwrap();
        assert_eq!(&bytes[0..2], b"MZ");
    }

    // Test 2: PE signature at offset 64
    #[test]
    fn pe_signature_at_64() {
        let bytes = pack(&win64_art()).unwrap();
        assert_eq!(&bytes[64..68], b"PE\x00\x00");
    }

    // Test 3: rejects linux_x64
    #[test]
    fn rejects_linux_target() {
        let art = CodeArtifact::new(vec![0x90], 0, Target::linux_x64());
        assert!(matches!(pack(&art), Err(PackagerError::UnsupportedTarget(_))));
    }

    // Test 4: rejects macos_x64
    #[test]
    fn rejects_macos_target() {
        let art = CodeArtifact::new(vec![0x90], 0, Target::macos_x64());
        assert!(matches!(pack(&art), Err(PackagerError::UnsupportedTarget(_))));
    }

    // Test 5: file size is 0x200 + aligned code
    #[test]
    fn file_size_correct() {
        // 1 byte of code → aligned to 512 → total 0x200 + 0x200 = 0x400
        let bytes = pack(&win64_art()).unwrap();
        assert_eq!(bytes.len(), 0x400);
    }

    // Test 6: native bytes at 0x200
    #[test]
    fn code_at_0x200() {
        let bytes = pack(&win64_art()).unwrap();
        assert_eq!(bytes[0x200], 0xC3); // RET
    }

    // Test 7: Machine field in COFF header = 0x8664
    #[test]
    fn coff_machine_amd64() {
        let bytes = pack(&win64_art()).unwrap();
        // COFF header starts at offset 68. Machine is the first field (2 bytes).
        let machine = u16::from_le_bytes([bytes[68], bytes[69]]);
        assert_eq!(machine, 0x8664);
    }

    // Test 8: align_to helper
    #[test]
    fn align_to_helper() {
        assert_eq!(align_to(0, 512), 0);
        assert_eq!(align_to(1, 512), 512);
        assert_eq!(align_to(512, 512), 512);
        assert_eq!(align_to(513, 512), 1024);
        assert_eq!(align_to(1000, 4096), 4096);
    }

    // Test 9: file_extension
    #[test]
    fn file_extension_is_exe() {
        assert_eq!(file_extension(), ".exe");
    }

    // Test 10: optional header magic = 0x020B (PE32+)
    #[test]
    fn optional_header_magic_pe32plus() {
        let bytes = pack(&win64_art()).unwrap();
        // Optional header starts at 68 + 20 = 88. First 2 bytes = magic.
        let magic = u16::from_le_bytes([bytes[88], bytes[89]]);
        assert_eq!(magic, 0x020B, "expected PE32+ magic 0x020B, got {magic:#06X}");
    }
}
