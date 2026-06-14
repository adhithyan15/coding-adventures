//! PE/COFF relocatable **object file** writer for Windows x86-64.
//!
//! Where [`crate::pe`] produces a fully-linked Windows PE32+ executable,
//! this module produces a relocatable COFF object file (no MZ stub,
//! no optional header) intended to be handed to the system linker
//! (`link.exe` / `lld-link.exe`).  Implements the PE/COFF half of
//! [LANG45](../../../../specs/LANG45-x86_64-object-formats.md).
//!
//! ## Why an object-file emitter
//!
//! Same rationale as `elf_object`: Twig programs call into
//! `__twig_print_i64`, which on Windows resolves through the C runtime
//! library.  Producing a self-contained `.exe` that dynamically links
//! `ucrtbase.dll` / `msvcrt.dll` is many lines of glue (import
//! directory, IAT, base relocations).  Delegating the final link to
//! `link.exe` is dramatically simpler, and matches the pattern
//! `macho_object` already uses for macOS ARM64.
//!
//! ## File layout
//!
//! ```text
//! Offset                  │ Size │ Content
//! ────────────────────────┼──────┼──────────────────────────────────────────
//! 0                       │  20  │ IMAGE_FILE_HEADER
//! 20                      │  40  │ IMAGE_SECTION_HEADER for .text
//! [60                     │  40] │ IMAGE_SECTION_HEADER for .data (if globals)
//! aligned                 │  T   │ .text raw bytes
//! aligned                 │  D   │ .data raw bytes (zero-init globals)
//! [aligned                │ 10·R]│ IMAGE_RELOCATION entries for .text
//! aligned                 │ 18·N │ IMAGE_SYMBOL entries
//! aligned                 │  4+S │ COFF string table (4-byte length + names)
//! ```
//!
//! Note: PE/COFF object files have **no MZ/DOS stub** — that prefix is
//! used only by executable files (`.exe`).
//!
//! ## Relocations
//!
//! | Abstract kind | PE/COFF type | Hex |
//! |---|---|---|
//! | `PltRel32` | `IMAGE_REL_AMD64_REL32` | 0x04 |
//! | `PcRel32` | `IMAGE_REL_AMD64_REL32` | 0x04 |
//! | `GotPcRel32` | `IMAGE_REL_AMD64_REL32` | 0x04 (no GOT on Windows) |
//!
//! Unlike ELF, PE/COFF relocs do **not** carry an explicit addend.
//! The linker computes:
//!
//! ```text
//! patched_value = target_address - (patch_offset + 4) + existing_bytes_at_patch
//! ```
//!
//! `x86_64-encoder` writes zero into the disp32 slot, so the result is
//! `target - end_of_instruction` — the same correct PC-relative delta
//! that ELF's `R_X86_64_PC32` with `addend=-4` produces.
//!
//! ## Symbol-name conventions
//!
//! On 64-bit Windows, MSVC's `link.exe` does **not** prepend an
//! underscore to C symbol names (unlike 32-bit Win32).  So:
//!
//! - Entry point is `main` (the CRT's `mainCRTStartup` calls it
//!   directly).
//! - Runtime helper is `__twig_print_i64` (the leading double
//!   underscore is part of the source name, not a calling-convention
//!   decoration).
//!
//! Names ≤ 8 characters are inlined in the IMAGE_SYMBOL.Name field;
//! longer names are placed in the string table and referenced with a
//! 4-byte offset.
//!
//! ## Reference
//!
//! - *Microsoft PE and COFF Specification*, revision 8.3
//! - `<winnt.h>` constants

use crate::errors::PackagerError;
use crate::target::Target;
use crate::elf_object::{X86RelocKind, X86RelocRecord};

// ---- Sizes ------------------------------------------------------------------

const FILE_HEADER_SIZE: u64 = 20;
const SECTION_HEADER_SIZE: u64 = 40;
const RELOCATION_SIZE: u64 = 10;
const SYMBOL_SIZE: u64 = 18;

// ---- Constants (subset of <winnt.h>) ---------------------------------------

const IMAGE_FILE_MACHINE_AMD64: u16 = 0x8664;

// Section characteristics
const IMAGE_SCN_CNT_CODE:             u32 = 0x0000_0020;
const IMAGE_SCN_CNT_INITIALIZED_DATA: u32 = 0x0000_0040;
const IMAGE_SCN_ALIGN_8BYTES:         u32 = 0x0040_0000;
const IMAGE_SCN_ALIGN_16BYTES:        u32 = 0x0050_0000;
const IMAGE_SCN_MEM_EXECUTE:          u32 = 0x2000_0000;
const IMAGE_SCN_MEM_READ:             u32 = 0x4000_0000;
const IMAGE_SCN_MEM_WRITE:            u32 = 0x8000_0000;

// Composed flag masks for our two section kinds.
const TEXT_CHARACTERISTICS: u32 =
    IMAGE_SCN_CNT_CODE | IMAGE_SCN_MEM_EXECUTE | IMAGE_SCN_MEM_READ
    | IMAGE_SCN_ALIGN_16BYTES;
const DATA_CHARACTERISTICS: u32 =
    IMAGE_SCN_CNT_INITIALIZED_DATA | IMAGE_SCN_MEM_READ | IMAGE_SCN_MEM_WRITE
    | IMAGE_SCN_ALIGN_8BYTES;

// Relocation types
const IMAGE_REL_AMD64_REL32: u16 = 0x04;

// Symbol storage classes
const IMAGE_SYM_CLASS_EXTERNAL: u8 = 2;

// Symbol type bits (Type field, high byte = "complex type")
const IMAGE_SYM_DTYPE_FUNCTION: u16 = 0x20;

// SectionNumber sentinel for undefined externs.
const IMAGE_SYM_UNDEFINED: i16 = 0;

// Entry point and globals symbol names.
const ENTRY_SYMBOL: &str = "main";
const GLOBALS_SYMBOL: &str = "_twig_globals";

// ---- Public entry point -----------------------------------------------------

/// Pack a single concatenated `.text` byte stream + globals slab + the
/// linker-resolved relocations into a Windows x86-64 PE/COFF object
/// file (`*.obj`) suitable for handing to `link.exe` / `lld-link.exe`.
///
/// Returns the raw object-file bytes.
///
/// # Parameters
///
/// - `text_bytes` — concatenated machine code for the `.text` section.
/// - `entry_point` — byte offset of `main` within `text_bytes`.
/// - `n_global_slots` — number of 8-byte global variable slots.  The
///   `.data` section is `n_global_slots * 8` bytes of zeros.  Pass `0`
///   to omit the `.data` section entirely.
/// - `relocs` — relocation records produced by the backend (reuses the
///   neutral `X86RelocRecord` type from [`crate::elf_object`]).
/// - `target` — must be `windows` + `x86_64`.
pub fn pack_pe_object_x86_64(
    text_bytes: &[u8],
    entry_point: usize,
    n_global_slots: usize,
    relocs: &[X86RelocRecord],
    target: &Target,
) -> Result<Vec<u8>, PackagerError> {
    if target.arch != "x86_64" {
        return Err(PackagerError::UnsupportedTarget(format!(
            "pack_pe_object_x86_64: expected arch=x86_64, got {:?}",
            target.arch
        )));
    }
    if target.os != "windows" {
        return Err(PackagerError::UnsupportedTarget(format!(
            "pack_pe_object_x86_64: expected os=windows, got {:?}",
            target.os
        )));
    }

    let code_len = text_bytes.len() as u64;
    if entry_point as u64 > code_len {
        return Err(PackagerError::UnsupportedTarget(format!(
            "pack_pe_object_x86_64: entry_point {} exceeds text length {}",
            entry_point, code_len
        )));
    }

    let data_len = (n_global_slots as u64) * 8;
    let have_data = n_global_slots > 0;
    let have_relocs = !relocs.is_empty();

    // -- Unique externs (preserve order of first appearance) ----------------
    let mut unique_exts: Vec<&str> = Vec::new();
    {
        use std::collections::HashSet;
        let mut seen: HashSet<&str> = HashSet::new();
        for r in relocs {
            if seen.insert(r.symbol.as_str()) {
                unique_exts.push(&r.symbol);
            }
        }
    }

    // -- Section numbering --------------------------------------------------
    //
    // PE/COFF SectionNumber is 1-based.  Always: .text = 1.  .data = 2 only
    // if globals are present.
    let n_sections: u16 = 1 + if have_data { 1 } else { 0 };
    let text_section_num: u16 = 1;
    let data_section_num: u16 = if have_data { 2 } else { 0 };

    // -- Symbol numbering ---------------------------------------------------
    //
    // Index 0 = main
    // Index 1 = _twig_globals (if have_data)
    // Index N+ = unique externs
    let n_syms: u64 = 1
        + if have_data { 1 } else { 0 }
        + unique_exts.len() as u64;
    let first_ext_sym_idx: u32 = 1 + if have_data { 1 } else { 0 };

    // -- String table -------------------------------------------------------
    //
    // PE/COFF symbol names ≤ 8 chars are inlined.  Longer names go to the
    // string table, referenced from IMAGE_SYMBOL.Name as 4 zero bytes
    // followed by a 4-byte string-table offset.  String table starts with
    // a 4-byte length (including the 4 length bytes themselves).
    let mut strtab: Vec<u8> = Vec::new();
    strtab.extend_from_slice(&[0u8; 4]); // length placeholder, filled later

    /// Convert a symbol name into its IMAGE_SYMBOL.Name 8-byte field.
    /// Appends to `strtab` if longer than 8 chars.
    fn pe_name_field(name: &str, strtab: &mut Vec<u8>) -> [u8; 8] {
        if name.len() <= 8 {
            let mut buf = [0u8; 8];
            buf[..name.len()].copy_from_slice(name.as_bytes());
            buf
        } else {
            let off = strtab.len() as u32;
            strtab.extend_from_slice(name.as_bytes());
            strtab.push(0u8);
            let mut buf = [0u8; 8];
            buf[..4].copy_from_slice(&0u32.to_le_bytes()); // 4 zero bytes
            buf[4..].copy_from_slice(&off.to_le_bytes());
            buf
        }
    }

    let main_name = pe_name_field(ENTRY_SYMBOL, &mut strtab);
    let globals_name = if have_data {
        pe_name_field(GLOBALS_SYMBOL, &mut strtab)
    } else { [0u8; 8] };
    let ext_names: Vec<[u8; 8]> = unique_exts.iter()
        .map(|name| pe_name_field(name, &mut strtab))
        .collect();

    // Patch the length prefix of the string table.
    let strtab_len = strtab.len() as u32;
    strtab[0..4].copy_from_slice(&strtab_len.to_le_bytes());

    // -- File-offset calculation --------------------------------------------
    //
    // Layout: file_header → section headers → raw section data → relocs
    //       → symbol table → string table.
    //
    // PE/COFF doesn't require strict alignment between sections in object
    // files, but `link.exe` is happier with a clean 4-byte alignment for
    // section data.
    let align_up = |x: u64, a: u64| -> u64 { (x + a - 1) & !(a - 1) };

    let section_headers_off: u64 = FILE_HEADER_SIZE;
    let text_data_off: u64 = align_up(
        section_headers_off + SECTION_HEADER_SIZE * n_sections as u64, 4);
    let mut cursor = text_data_off + code_len;
    let data_data_off = if have_data {
        cursor = align_up(cursor, 8);
        let o = cursor;
        cursor += data_len;
        o
    } else { 0 };
    let text_relocs_off = if have_relocs {
        cursor = align_up(cursor, 4);
        let o = cursor;
        cursor += RELOCATION_SIZE * relocs.len() as u64;
        o
    } else { 0 };
    let symtab_off = {
        cursor = align_up(cursor, 4);
        let o = cursor;
        cursor += SYMBOL_SIZE * n_syms;
        o
    };
    let strtab_off = {
        let o = cursor;
        cursor += strtab.len() as u64;
        o
    };
    let total_size = cursor as usize;

    // -- Build the file ------------------------------------------------------
    let mut out: Vec<u8> = Vec::with_capacity(total_size);

    // IMAGE_FILE_HEADER (20 bytes)
    out.extend_from_slice(&IMAGE_FILE_MACHINE_AMD64.to_le_bytes());
    out.extend_from_slice(&n_sections.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // TimeDateStamp (reproducible)
    out.extend_from_slice(&(symtab_off as u32).to_le_bytes()); // PointerToSymbolTable
    out.extend_from_slice(&(n_syms as u32).to_le_bytes()); // NumberOfSymbols
    out.extend_from_slice(&0u16.to_le_bytes()); // SizeOfOptionalHeader (object: 0)
    out.extend_from_slice(&0u16.to_le_bytes()); // Characteristics
    debug_assert_eq!(out.len() as u64, FILE_HEADER_SIZE);

    // IMAGE_SECTION_HEADER for .text
    write_section_header(
        &mut out, b".text\0\0\0",
        code_len as u32,
        text_data_off as u32,
        if have_relocs { text_relocs_off as u32 } else { 0 },
        if have_relocs { relocs.len() as u16 } else { 0 },
        TEXT_CHARACTERISTICS);

    // IMAGE_SECTION_HEADER for .data (if present)
    if have_data {
        write_section_header(
            &mut out, b".data\0\0\0",
            data_len as u32,
            data_data_off as u32,
            0, // PointerToRelocations — no relocs in .data for V1
            0,
            DATA_CHARACTERISTICS);
    }

    // .text raw bytes
    pad_to(&mut out, text_data_off);
    out.extend_from_slice(text_bytes);

    // .data raw bytes (zero-init)
    if have_data {
        pad_to(&mut out, data_data_off);
        out.resize(out.len() + data_len as usize, 0u8);
    }

    // .text IMAGE_RELOCATION records
    if have_relocs {
        pad_to(&mut out, text_relocs_off);
        for r in relocs {
            let sym_idx: u32 = if r.symbol == ENTRY_SYMBOL {
                0
            } else if r.symbol == GLOBALS_SYMBOL && have_data {
                1
            } else {
                // Find in unique externs
                first_ext_sym_idx + unique_exts.iter()
                    .position(|s| *s == r.symbol)
                    .expect("symbol must appear in unique_exts or be main/_twig_globals")
                    as u32
            };
            let pe_type = match r.kind {
                X86RelocKind::PltRel32   |
                X86RelocKind::PcRel32    |
                X86RelocKind::GotPcRel32 => IMAGE_REL_AMD64_REL32,
            };
            out.extend_from_slice(&r.patch_offset.to_le_bytes());
            out.extend_from_slice(&sym_idx.to_le_bytes());
            out.extend_from_slice(&pe_type.to_le_bytes());
        }
    }

    // Symbol table
    pad_to(&mut out, symtab_off);
    // Symbol 0: main
    write_symbol(
        &mut out, &main_name,
        entry_point as u32,
        text_section_num as i16,
        IMAGE_SYM_DTYPE_FUNCTION,
        IMAGE_SYM_CLASS_EXTERNAL);
    // Symbol 1: _twig_globals (if have_data)
    if have_data {
        write_symbol(
            &mut out, &globals_name,
            0,
            data_section_num as i16,
            0,
            IMAGE_SYM_CLASS_EXTERNAL);
    }
    // Remaining: unique externs
    for (i, _) in unique_exts.iter().enumerate() {
        write_symbol(
            &mut out, &ext_names[i],
            0,
            IMAGE_SYM_UNDEFINED,
            0,
            IMAGE_SYM_CLASS_EXTERNAL);
    }

    // String table
    pad_to(&mut out, strtab_off);
    out.extend_from_slice(&strtab);

    debug_assert_eq!(out.len(), total_size,
        "pe_object: emitted {} bytes, expected {total_size}", out.len());

    Ok(out)
}

// ---- Helpers ----------------------------------------------------------------

fn pad_to(out: &mut Vec<u8>, offset: u64) {
    let cur = out.len() as u64;
    debug_assert!(cur <= offset, "pad_to: cursor {cur} already past target {offset}");
    if cur < offset {
        out.resize(offset as usize, 0u8);
    }
}

/// Write an IMAGE_SECTION_HEADER (40 bytes).
fn write_section_header(
    out: &mut Vec<u8>,
    name8: &[u8; 8],
    size_of_raw_data: u32,
    pointer_to_raw_data: u32,
    pointer_to_relocations: u32,
    number_of_relocations: u16,
    characteristics: u32,
) {
    out.extend_from_slice(name8);
    out.extend_from_slice(&0u32.to_le_bytes()); // VirtualSize (object: 0)
    out.extend_from_slice(&0u32.to_le_bytes()); // VirtualAddress (object: 0)
    out.extend_from_slice(&size_of_raw_data.to_le_bytes());
    out.extend_from_slice(&pointer_to_raw_data.to_le_bytes());
    out.extend_from_slice(&pointer_to_relocations.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // PointerToLinenumbers
    out.extend_from_slice(&number_of_relocations.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // NumberOfLinenumbers
    out.extend_from_slice(&characteristics.to_le_bytes());
}

/// Write an IMAGE_SYMBOL (18 bytes).
fn write_symbol(
    out: &mut Vec<u8>,
    name8: &[u8; 8],
    value: u32,
    section_number: i16,
    sym_type: u16,
    storage_class: u8,
) {
    out.extend_from_slice(name8);
    out.extend_from_slice(&value.to_le_bytes());
    out.extend_from_slice(&section_number.to_le_bytes());
    out.extend_from_slice(&sym_type.to_le_bytes());
    out.push(storage_class);
    out.push(0u8); // NumberOfAuxSymbols
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn windows_x64() -> Target { Target::windows_x64() }

    #[test]
    fn rejects_wrong_arch() {
        let mut t = windows_x64();
        t.arch = "arm64".to_string();
        let r = pack_pe_object_x86_64(b"\xC3", 0, 0, &[], &t);
        assert!(r.is_err());
    }

    #[test]
    fn rejects_wrong_os() {
        let t = Target::linux_x64();
        let r = pack_pe_object_x86_64(b"\xC3", 0, 0, &[], &t);
        assert!(r.is_err());
    }

    #[test]
    fn rejects_entry_past_text() {
        let r = pack_pe_object_x86_64(b"\xC3", 99, 0, &[], &windows_x64());
        assert!(r.is_err());
    }

    #[test]
    fn minimal_object_has_machine_field() {
        let out = pack_pe_object_x86_64(b"\xC3", 0, 0, &[], &windows_x64()).unwrap();
        // First 2 bytes: IMAGE_FILE_MACHINE_AMD64 = 0x8664 (little-endian)
        assert_eq!(&out[0..2], &[0x64, 0x86]);
    }

    #[test]
    fn no_mz_stub() {
        // Object files do NOT start with the "MZ" DOS signature — that's
        // executable-only.  The third byte (NumberOfSections low byte) is
        // the section count, which for our minimal object is 1.
        let out = pack_pe_object_x86_64(b"\xC3", 0, 0, &[], &windows_x64()).unwrap();
        // Byte 0..2 = Machine (0x8664), byte 2..4 = NumberOfSections (= 1)
        assert_eq!(&out[2..4], &[0x01, 0x00]);
    }

    #[test]
    fn one_section_when_no_globals() {
        let out = pack_pe_object_x86_64(b"\xC3", 0, 0, &[], &windows_x64()).unwrap();
        assert_eq!(&out[2..4], &[0x01, 0x00]); // NumberOfSections = 1
    }

    #[test]
    fn two_sections_when_globals_present() {
        let out = pack_pe_object_x86_64(b"\xC3", 0, 4, &[], &windows_x64()).unwrap();
        assert_eq!(&out[2..4], &[0x02, 0x00]); // NumberOfSections = 2
    }

    #[test]
    fn text_section_header_at_offset_20() {
        // First section header (.text) starts at byte 20.
        let out = pack_pe_object_x86_64(b"\xC3", 0, 0, &[], &windows_x64()).unwrap();
        // Section name bytes 0..8 — should be ".text\0\0\0".
        assert_eq!(&out[20..28], b".text\0\0\0");
    }

    #[test]
    fn data_section_header_when_globals() {
        let out = pack_pe_object_x86_64(b"\xC3", 0, 4, &[], &windows_x64()).unwrap();
        // .data section header starts at byte 60 (20 + 40).
        assert_eq!(&out[60..68], b".data\0\0\0");
    }

    #[test]
    fn text_bytes_appear_after_section_headers() {
        let text = b"\x48\x31\xC0\xC3"; // xor rax, rax; ret
        let out = pack_pe_object_x86_64(text, 0, 0, &[], &windows_x64()).unwrap();
        // PointerToRawData is at section_header_off + 20 (offset within section header).
        // section header for .text starts at byte 20, so PointerToRawData is at byte 40.
        let ptr_to_raw = u32::from_le_bytes([out[40], out[41], out[42], out[43]]);
        assert!(ptr_to_raw >= 60); // must be past the section headers
        assert_eq!(&out[ptr_to_raw as usize..ptr_to_raw as usize + text.len()], text);
    }

    #[test]
    fn relocs_emit_rel32_type() {
        let text = vec![0xE8, 0, 0, 0, 0];
        let relocs = vec![X86RelocRecord {
            patch_offset: 1,
            symbol: "__twig_print_i64".into(),
            kind: X86RelocKind::PltRel32,
            addend: -4,
        }];
        let out = pack_pe_object_x86_64(&text, 0, 0, &relocs, &windows_x64()).unwrap();
        // IMAGE_REL_AMD64_REL32 = 0x04 — must appear somewhere in the reloc records.
        // The reloc is 10 bytes: 4 (offset) + 4 (sym_idx) + 2 (type).
        // Search for the 10-byte pattern: offset=1, sym_idx=1, type=4.
        // (sym 0 = main, sym 1 = __twig_print_i64 since no globals)
        let needle = [
            0x01, 0, 0, 0,   // VirtualAddress = 1
            0x01, 0, 0, 0,   // SymbolTableIndex = 1
            0x04, 0,         // Type = IMAGE_REL_AMD64_REL32
        ];
        assert!(out.windows(needle.len()).any(|w| w == needle),
                "expected reloc record {needle:02X?} in {out:02X?}");
    }

    #[test]
    fn long_symbol_uses_string_table() {
        // "__twig_print_i64" (16 chars) must be in the string table —
        // its IMAGE_SYMBOL.Name should be 4 zero bytes + 4-byte offset.
        let text = vec![0xE8, 0, 0, 0, 0];
        let relocs = vec![X86RelocRecord {
            patch_offset: 1,
            symbol: "__twig_print_i64".into(),
            kind: X86RelocKind::PltRel32,
            addend: -4,
        }];
        let out = pack_pe_object_x86_64(&text, 0, 0, &relocs, &windows_x64()).unwrap();
        // The name "__twig_print_i64" should appear in the string table.
        let needle = b"__twig_print_i64";
        assert!(out.windows(needle.len()).any(|w| w == needle));
    }

    #[test]
    fn short_symbol_inlined() {
        // "main" is 4 chars — should appear inline in the symbol table
        // (not in the string table).  We verify by checking that the
        // sequence b"main\0\0\0\0" appears as a contiguous 8-byte block,
        // which only happens if it's inlined as IMAGE_SYMBOL.Name.
        let out = pack_pe_object_x86_64(b"\xC3", 0, 0, &[], &windows_x64()).unwrap();
        let needle = b"main\0\0\0\0";
        assert!(out.windows(needle.len()).any(|w| w == needle));
    }

    #[test]
    fn main_symbol_table_index_is_zero() {
        // First symbol in the symtab must be main, so a reloc referencing
        // it (if we had one) would carry sym_idx=0.  We don't emit such
        // a reloc directly, but we can verify the symbol's value field
        // holds the entry_point.  Find main's inline name and read the
        // following 4-byte Value field.
        let entry = 7u32;
        // Text long enough to contain entry offset 7.
        let mut text = vec![0x90u8; 10];
        text[7] = 0xC3;
        let out = pack_pe_object_x86_64(&text, entry as usize, 0, &[], &windows_x64()).unwrap();
        // Find b"main\0\0\0\0" — the bytes immediately following are Value (u32 LE).
        let pos = out.windows(8).position(|w| w == b"main\0\0\0\0").unwrap();
        let value = u32::from_le_bytes([
            out[pos + 8], out[pos + 9], out[pos + 10], out[pos + 11]
        ]);
        assert_eq!(value, entry);
    }

    #[test]
    fn unique_external_symbols_deduped() {
        let text = vec![0xE8, 0, 0, 0, 0, 0xE8, 0, 0, 0, 0];
        let relocs = vec![
            X86RelocRecord {
                patch_offset: 1,
                symbol: "foo".into(),
                kind: X86RelocKind::PltRel32,
                addend: -4,
            },
            X86RelocRecord {
                patch_offset: 6,
                symbol: "foo".into(),
                kind: X86RelocKind::PltRel32,
                addend: -4,
            },
        ];
        let out = pack_pe_object_x86_64(&text, 0, 0, &relocs, &windows_x64()).unwrap();
        // "foo" is 3 chars → inlined in IMAGE_SYMBOL.Name as b"foo\0\0\0\0\0".
        // Should appear exactly once (one symbol, two relocs reference it).
        let needle = b"foo\0\0\0\0\0";
        let count = out.windows(needle.len()).filter(|w| *w == needle).count();
        assert_eq!(count, 1, "expected exactly one inlined 'foo' symbol");
    }

    #[test]
    fn n_symbols_field_correct() {
        // No globals, no externs → just 'main' → NumberOfSymbols = 1
        let out = pack_pe_object_x86_64(b"\xC3", 0, 0, &[], &windows_x64()).unwrap();
        // NumberOfSymbols is at byte offset 12 of IMAGE_FILE_HEADER (u32 LE).
        let n_syms = u32::from_le_bytes([out[12], out[13], out[14], out[15]]);
        assert_eq!(n_syms, 1);
    }

    #[test]
    fn n_symbols_with_globals_and_externs() {
        let text = vec![0xE8, 0, 0, 0, 0];
        let relocs = vec![X86RelocRecord {
            patch_offset: 1,
            symbol: "foo".into(),
            kind: X86RelocKind::PltRel32,
            addend: -4,
        }];
        let out = pack_pe_object_x86_64(&text, 0, 4, &relocs, &windows_x64()).unwrap();
        // main + _twig_globals + foo = 3
        let n_syms = u32::from_le_bytes([out[12], out[13], out[14], out[15]]);
        assert_eq!(n_syms, 3);
    }
}
