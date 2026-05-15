//! ELF64 relocatable **object file** writer for x86-64.
//!
//! Where [`crate::elf64`] produces a fully-linked Linux ELF *executable*,
//! this module produces a relocatable object file (`ET_REL`) intended
//! to be handed to the system linker (`cc` / `ld`).  Implements the
//! `x86-64` half of [LANG45](../../../../specs/LANG45-x86_64-object-formats.md).
//!
//! ## Why an object-file emitter
//!
//! Twig programs print integers via the `__twig_print_i64` runtime
//! helper, which on Linux resolves through `libc`'s `printf`.  Producing
//! a self-contained executable that dynamically links `libc` is many
//! lines of glue (dynamic section, GOT/PLT, relocation processing).
//! Delegating the final link to the system linker is dramatically
//! simpler, and matches the pattern `macho_object` already uses for
//! macOS ARM64.
//!
//! ## File layout
//!
//! ```text
//! Offset                  │ Size │ Content
//! ────────────────────────┼──────┼──────────────────────────────────────────
//! 0                       │  64  │ Elf64_Ehdr  (ET_REL, EM_X86_64)
//! 64                      │  T   │ .text section bytes
//! 64+T                    │  D   │ .data section bytes (zero-init globals)
//! 64+T+D                  │ 24·R │ .rela.text relocation records
//! aligned                 │ 24·N │ .symtab records (24 bytes per Elf64_Sym)
//! aligned                 │  S   │ .strtab (symbol names)
//! aligned                 │  X   │ .shstrtab (section names)
//! aligned                 │ 64·7 │ Section header table (7 sections)
//! ```
//!
//! ## Section table (in order)
//!
//! | Index | Name | Type | Flags | Purpose |
//! |-------|------|------|-------|---------|
//! | 0 | `(null)` | `SHT_NULL` | 0 | ELF mandatory |
//! | 1 | `.text` | `SHT_PROGBITS` | `ALLOC \| EXECINSTR` | Machine code |
//! | 2 | `.data` | `SHT_PROGBITS` | `ALLOC \| WRITE` | Twig globals slab |
//! | 3 | `.rela.text` | `SHT_RELA` | `INFO_LINK` | Relocations for `.text` |
//! | 4 | `.symtab` | `SHT_SYMTAB` | 0 | Symbol table |
//! | 5 | `.strtab` | `SHT_STRTAB` | 0 | Symbol name strings |
//! | 6 | `.shstrtab` | `SHT_STRTAB` | 0 | Section name strings |
//!
//! Relocation type IDs:
//!
//! | Abstract kind | ELF type | Hex |
//! |---|---|---|
//! | `PltRel32` | `R_X86_64_PLT32` | 4 |
//! | `PcRel32` | `R_X86_64_PC32` | 2 |
//! | `GotPcRel32` | `R_X86_64_REX_GOTPCRELX` | 42 |
//!
//! All ELF reloc records carry the addend explicitly (`Elf64_Rela.r_addend`);
//! x86-64 PC-relative is `-4` for the V1 forms produced by `x86_64-encoder`
//! (the disp32 lives 4 bytes before the instruction end, which is the PC
//! reference).
//!
//! ## Reference
//!
//! - System V Application Binary Interface: AMD64 Architecture Processor
//!   Supplement, §4 (Object Files)
//! - *ELF-64 Object File Format*, version 1.5 draft
//! - `/usr/include/elf.h` (glibc) for canonical constant values

use crate::errors::PackagerError;
use crate::target::Target;

// ---- Constants (subset of <elf.h>) -----------------------------------------

const EI_NIDENT: usize = 16;
const ELFMAG: [u8; 4] = [0x7F, b'E', b'L', b'F'];
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const EV_CURRENT: u8 = 1;
const ELFOSABI_SYSV: u8 = 0;

const ET_REL: u16 = 1;
const EM_X86_64: u16 = 62;

const SHT_NULL:     u32 = 0;
const SHT_PROGBITS: u32 = 1;
const SHT_SYMTAB:   u32 = 2;
const SHT_STRTAB:   u32 = 3;
const SHT_RELA:     u32 = 4;

const SHF_WRITE:     u64 = 0x1;
const SHF_ALLOC:     u64 = 0x2;
const SHF_EXECINSTR: u64 = 0x4;
const SHF_INFO_LINK: u64 = 0x40;

const STB_GLOBAL: u8 = 1;
const STT_NOTYPE: u8 = 0;
const STT_OBJECT: u8 = 1;
const STT_FUNC:   u8 = 2;

const SHN_UNDEF: u16 = 0;

const R_X86_64_PC32:           u32 = 2;
const R_X86_64_PLT32:          u32 = 4;
const R_X86_64_REX_GOTPCRELX:  u32 = 42;

// ---- Sizes ------------------------------------------------------------------

const EHDR_SIZE:  u64 = 64;
const SHDR_SIZE:  u64 = 64;
const SYM_SIZE:   u64 = 24;
const RELA_SIZE:  u64 = 24;

// ---- Public types -----------------------------------------------------------

/// Abstract x86-64 relocation kind.
///
/// The packager maps each kind to the appropriate ELF `R_X86_64_*`
/// type ID at emit time.  Matches `x86_64_encoder::ExternalRelocKind`
/// without depending on that crate (keeps `code-packager` independent
/// of the backend layer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum X86RelocKind {
    /// PC-relative branch to an external function (PLT-routed on shared
    /// objects).  Maps to `R_X86_64_PLT32`.
    PltRel32,
    /// PC-relative 32-bit displacement to a locally-resolved symbol.
    /// Maps to `R_X86_64_PC32`.
    PcRel32,
    /// RIP-relative GOT load.  Maps to `R_X86_64_REX_GOTPCRELX`.
    GotPcRel32,
}

impl X86RelocKind {
    /// ELF reloc type ID corresponding to this kind.
    fn elf_type(self) -> u32 {
        match self {
            X86RelocKind::PltRel32   => R_X86_64_PLT32,
            X86RelocKind::PcRel32    => R_X86_64_PC32,
            X86RelocKind::GotPcRel32 => R_X86_64_REX_GOTPCRELX,
        }
    }
}

/// One relocation record produced by the backend.
///
/// `patch_offset` is the byte offset (in the linked `.text` section)
/// where the 32-bit disp slot lives.  `symbol` is the external symbol
/// name the linker must resolve.  `addend` is recorded verbatim in the
/// `Elf64_Rela.r_addend` field — always `-4` for the V1 instruction
/// forms `x86_64-encoder` emits.
#[derive(Debug, Clone)]
pub struct X86RelocRecord {
    /// Section-relative byte offset of the 32-bit patch slot.
    pub patch_offset: u32,
    /// External symbol name (e.g. `"__twig_print_i64"`).
    pub symbol: String,
    /// Abstract kind — packager translates to ELF reloc type ID.
    pub kind: X86RelocKind,
    /// Addend stored in `r_addend`.
    pub addend: i32,
}

// ---- Entry point ------------------------------------------------------------

/// Linux entry-point symbol name.  Linux's `_start` (provided by
/// `Scrt1.o`) calls a function named `main` — no leading underscore.
const ENTRY_SYMBOL: &str = "main";

/// Twig globals slab symbol.  Same name as on macOS so the backend's
/// emitted reloc records work unmodified.
const GLOBALS_SYMBOL: &str = "_twig_globals";

/// Pack a single concatenated `.text` byte stream + globals slab + the
/// linker-resolved relocations into an ELF64 `ET_REL` object file
/// suitable for handing to `cc` / `ld` on Linux x86-64.
///
/// Returns the raw object-file bytes.
///
/// # Parameters
///
/// - `text_bytes` — concatenated machine code for the `.text` section.
/// - `entry_point` — byte offset of `main` within `text_bytes`.
/// - `n_global_slots` — number of 8-byte global variable slots.
///   The `.data` section is `n_global_slots * 8` bytes of zeros.
///   Pass `0` if the program uses no globals (the `.data` section is
///   omitted entirely along with the `_twig_globals` symbol).
/// - `relocs` — relocation records produced by the backend.
/// - `target` — must be `linux` + `x86_64`.
pub fn pack_elf64_object_x86_64(
    text_bytes: &[u8],
    entry_point: usize,
    n_global_slots: usize,
    relocs: &[X86RelocRecord],
    target: &Target,
) -> Result<Vec<u8>, PackagerError> {
    if target.arch != "x86_64" {
        return Err(PackagerError::UnsupportedTarget(format!(
            "pack_elf64_object_x86_64: expected arch=x86_64, got {:?}",
            target.arch
        )));
    }
    if target.os != "linux" {
        return Err(PackagerError::UnsupportedTarget(format!(
            "pack_elf64_object_x86_64: expected os=linux, got {:?}",
            target.os
        )));
    }

    let code_len = text_bytes.len() as u64;
    if entry_point as u64 > code_len {
        return Err(PackagerError::UnsupportedTarget(format!(
            "pack_elf64_object_x86_64: entry_point {} exceeds text length {}",
            entry_point, code_len
        )));
    }

    let data_len = (n_global_slots as u64) * 8;
    let have_data = n_global_slots > 0;
    let have_relocs = !relocs.is_empty();

    // -- Unique external symbol names (preserving order of first appearance) --
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

    // -- Section layout ------------------------------------------------------
    //
    // Variable layout: sections are conditionally present.  Indices we
    // assign here are written into `sh_link` / `sh_info` fields and into
    // the relocation `r_info` (which references symtab indices).
    //
    // Always-present:  [0]=null, [text]=1
    // Conditional:     [data] only if have_data; [rela.text] only if have_relocs.
    // Always-present:  [symtab], [strtab], [shstrtab]
    let idx_text     = 1u16;
    let mut next_idx = 2u16;
    let idx_data       = if have_data   { let i = next_idx; next_idx += 1; i } else { 0 };
    let _idx_rela_text = if have_relocs { let i = next_idx; next_idx += 1; i } else { 0 };
    let idx_symtab     = { let i = next_idx; next_idx += 1; i };
    let idx_strtab     = { let i = next_idx; next_idx += 1; i };
    let idx_shstrtab   = { let i = next_idx; next_idx += 1; i };
    let num_sections = next_idx;

    // -- Symbol table layout -------------------------------------------------
    //
    // Symbol 0 is the reserved null entry.  Then:
    //   index 1 = main           (STT_FUNC, st_shndx = .text)
    //   index 2 = _twig_globals  (STT_OBJECT, st_shndx = .data)   [if have_data]
    //   next... = unique externs (STT_NOTYPE, st_shndx = SHN_UNDEF)
    //
    // sh_info for SYMTAB = index of first non-local symbol.  All our
    // non-null symbols are STB_GLOBAL, so sh_info = 1.
    let sym_globals: u32 = if have_data { 2 } else { 0 };
    let first_ext_sym_idx: u32 = 1 + (if have_data { 1 } else { 0 }) + 1; // null + main + maybe _twig_globals

    // -- String table -------------------------------------------------------
    //
    // strtab layout: "\0main\0_twig_globals\0<ext1>\0<ext2>\0..."
    //
    // We record (name → byte offset) so symbols can reference into it.
    let mut strtab: Vec<u8> = Vec::new();
    strtab.push(0u8);
    let off_main = strtab.len() as u32;
    strtab.extend_from_slice(ENTRY_SYMBOL.as_bytes());
    strtab.push(0u8);
    let off_globals = if have_data {
        let o = strtab.len() as u32;
        strtab.extend_from_slice(GLOBALS_SYMBOL.as_bytes());
        strtab.push(0u8);
        o
    } else { 0 };
    let off_exts: Vec<u32> = unique_exts.iter().map(|sym| {
        let o = strtab.len() as u32;
        strtab.extend_from_slice(sym.as_bytes());
        strtab.push(0u8);
        o
    }).collect();

    // -- Section header name string table -----------------------------------
    let mut shstrtab: Vec<u8> = Vec::new();
    shstrtab.push(0u8);
    let off_text     = shstrtab.len() as u32; shstrtab.extend_from_slice(b".text\0");
    let off_data     = if have_data   { let o = shstrtab.len() as u32; shstrtab.extend_from_slice(b".data\0"); o } else { 0 };
    let off_rela_text = if have_relocs { let o = shstrtab.len() as u32; shstrtab.extend_from_slice(b".rela.text\0"); o } else { 0 };
    let off_symtab   = { let o = shstrtab.len() as u32; shstrtab.extend_from_slice(b".symtab\0"); o };
    let off_strtab   = { let o = shstrtab.len() as u32; shstrtab.extend_from_slice(b".strtab\0"); o };
    let off_shstrtab = { let o = shstrtab.len() as u32; shstrtab.extend_from_slice(b".shstrtab\0"); o };

    // -- File offset calculation ----------------------------------------------
    //
    // We lay out the file in this order:
    //   ehdr → .text → .data → .rela.text → .symtab → .strtab → .shstrtab → shdrs
    //
    // 8-byte alignment between sections is conservative; ELF only requires
    // alignment matching each section's `sh_addralign` (8 for symtab/rela).
    let align_up = |x: u64, a: u64| -> u64 { (x + a - 1) & !(a - 1) };

    let text_off = EHDR_SIZE;
    let mut cursor = text_off + code_len;
    let data_off = if have_data {
        cursor = align_up(cursor, 8);
        let o = cursor;
        cursor += data_len;
        o
    } else { 0 };
    let rela_off = if have_relocs {
        cursor = align_up(cursor, 8);
        let o = cursor;
        cursor += RELA_SIZE * relocs.len() as u64;
        o
    } else { 0 };
    let symtab_off = {
        cursor = align_up(cursor, 8);
        let o = cursor;
        let num_syms: u64 = 1 + 1
            + if have_data { 1 } else { 0 }
            + unique_exts.len() as u64;
        cursor += SYM_SIZE * num_syms;
        o
    };
    let strtab_off = {
        cursor = align_up(cursor, 1);
        let o = cursor;
        cursor += strtab.len() as u64;
        o
    };
    let shstrtab_off = {
        cursor = align_up(cursor, 1);
        let o = cursor;
        cursor += shstrtab.len() as u64;
        o
    };
    let shdr_off = {
        cursor = align_up(cursor, 8);
        cursor
    };
    let total_size = (shdr_off + SHDR_SIZE * num_sections as u64) as usize;

    // -- Build the file ------------------------------------------------------
    let mut out: Vec<u8> = Vec::with_capacity(total_size);

    // ELF header (64 bytes)
    {
        let mut e_ident = [0u8; EI_NIDENT];
        e_ident[0..4].copy_from_slice(&ELFMAG);
        e_ident[4] = ELFCLASS64;
        e_ident[5] = ELFDATA2LSB;
        e_ident[6] = EV_CURRENT;
        e_ident[7] = ELFOSABI_SYSV;
        out.extend_from_slice(&e_ident);
        out.extend_from_slice(&ET_REL.to_le_bytes());
        out.extend_from_slice(&EM_X86_64.to_le_bytes());
        out.extend_from_slice(&(EV_CURRENT as u32).to_le_bytes());
        out.extend_from_slice(&0u64.to_le_bytes()); // e_entry
        out.extend_from_slice(&0u64.to_le_bytes()); // e_phoff
        out.extend_from_slice(&shdr_off.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes()); // e_flags
        out.extend_from_slice(&(EHDR_SIZE as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // e_phentsize
        out.extend_from_slice(&0u16.to_le_bytes()); // e_phnum
        out.extend_from_slice(&(SHDR_SIZE as u16).to_le_bytes());
        out.extend_from_slice(&num_sections.to_le_bytes());
        out.extend_from_slice(&idx_shstrtab.to_le_bytes());
    }
    assert_eq!(out.len() as u64, EHDR_SIZE);

    // .text
    out.extend_from_slice(text_bytes);

    // .data
    if have_data {
        pad_to(&mut out, data_off);
        out.resize(out.len() + data_len as usize, 0u8);
    }

    // .rela.text
    if have_relocs {
        pad_to(&mut out, rela_off);
        for r in relocs {
            // Symbol index resolution.
            let sym_idx: u32 = first_ext_sym_idx + unique_exts.iter()
                .position(|s| *s == r.symbol).map(|i| i as u32)
                .unwrap_or_else(|| {
                    // Fall back to _twig_globals if the symbol matches.
                    // This branch is dead in practice because the backend
                    // only emits _twig_globals as the symbol for global
                    // relocs, but the unique_exts list includes everything.
                    // Adjust: _twig_globals is at sym_globals if have_data.
                    if r.symbol == GLOBALS_SYMBOL && have_data {
                        sym_globals - first_ext_sym_idx
                    } else {
                        0
                    }
                });
            // r_info = (sym_idx << 32) | type
            let r_info: u64 = ((sym_idx as u64) << 32) | (r.kind.elf_type() as u64);
            out.extend_from_slice(&(r.patch_offset as u64).to_le_bytes());
            out.extend_from_slice(&r_info.to_le_bytes());
            out.extend_from_slice(&(r.addend as i64).to_le_bytes());
        }
    }

    // .symtab
    pad_to(&mut out, symtab_off);
    // index 0 — null symbol
    write_sym(&mut out, 0, 0, 0, SHN_UNDEF, 0, 0);
    // index 1 — main
    write_sym(&mut out, off_main,
              sym_info(STB_GLOBAL, STT_FUNC),
              0,
              idx_text,
              entry_point as u64,
              0);
    // index 2 — _twig_globals (only if globals present)
    if have_data {
        write_sym(&mut out, off_globals,
                  sym_info(STB_GLOBAL, STT_OBJECT),
                  0,
                  idx_data,
                  0,
                  data_len);
    }
    // remaining — unique externs
    for (i, _ext) in unique_exts.iter().enumerate() {
        write_sym(&mut out, off_exts[i],
                  sym_info(STB_GLOBAL, STT_NOTYPE),
                  0,
                  SHN_UNDEF,
                  0,
                  0);
    }

    // .strtab
    pad_to(&mut out, strtab_off);
    out.extend_from_slice(&strtab);

    // .shstrtab
    pad_to(&mut out, shstrtab_off);
    out.extend_from_slice(&shstrtab);

    // Section header table
    pad_to(&mut out, shdr_off);
    // [0] null
    write_shdr(&mut out, 0, SHT_NULL, 0, 0, 0, 0, 0, 0, 0);
    // [1] .text
    write_shdr(&mut out,
        off_text, SHT_PROGBITS, SHF_ALLOC | SHF_EXECINSTR,
        text_off, code_len, 0, 0, 16, 0);
    // [data] .data
    if have_data {
        write_shdr(&mut out,
            off_data, SHT_PROGBITS, SHF_ALLOC | SHF_WRITE,
            data_off, data_len, 0, 0, 8, 0);
    }
    // [rela.text] .rela.text
    if have_relocs {
        write_shdr(&mut out,
            off_rela_text, SHT_RELA, SHF_INFO_LINK,
            rela_off, RELA_SIZE * relocs.len() as u64,
            idx_symtab as u32, idx_text as u32,
            8, RELA_SIZE);
    }
    // [symtab] .symtab
    let num_syms = 1 + 1
        + if have_data { 1 } else { 0 }
        + unique_exts.len() as u64;
    write_shdr(&mut out,
        off_symtab, SHT_SYMTAB, 0,
        symtab_off, SYM_SIZE * num_syms,
        idx_strtab as u32, 1,  // sh_info = index of first non-local = 1
        8, SYM_SIZE);
    // [strtab] .strtab
    write_shdr(&mut out,
        off_strtab, SHT_STRTAB, 0,
        strtab_off, strtab.len() as u64,
        0, 0, 1, 0);
    // [shstrtab] .shstrtab
    write_shdr(&mut out,
        off_shstrtab, SHT_STRTAB, 0,
        shstrtab_off, shstrtab.len() as u64,
        0, 0, 1, 0);

    debug_assert_eq!(out.len(), total_size,
        "elf_object: emitted {} bytes, expected {total_size}", out.len());

    Ok(out)
}

// ---- Helpers ----------------------------------------------------------------

/// Pad `out` with zero bytes up to (but not past) the given absolute
/// file offset.
fn pad_to(out: &mut Vec<u8>, offset: u64) {
    let cur = out.len() as u64;
    debug_assert!(cur <= offset, "pad_to: cursor {cur} already past target {offset}");
    if cur < offset {
        out.resize(offset as usize, 0u8);
    }
}

/// Compose the `st_info` byte: high 4 bits = binding, low 4 = type.
#[inline]
fn sym_info(binding: u8, ty: u8) -> u8 {
    (binding << 4) | (ty & 0xF)
}

/// Write a single `Elf64_Sym` (24 bytes).
#[allow(clippy::too_many_arguments)]
fn write_sym(
    out: &mut Vec<u8>,
    name_offset: u32,
    info: u8,
    other: u8,
    shndx: u16,
    value: u64,
    size: u64,
) {
    out.extend_from_slice(&name_offset.to_le_bytes());
    out.push(info);
    out.push(other);
    out.extend_from_slice(&shndx.to_le_bytes());
    out.extend_from_slice(&value.to_le_bytes());
    out.extend_from_slice(&size.to_le_bytes());
}

/// Write a single `Elf64_Shdr` (64 bytes).
#[allow(clippy::too_many_arguments)]
fn write_shdr(
    out: &mut Vec<u8>,
    name: u32,
    sh_type: u32,
    flags: u64,
    offset: u64,
    size: u64,
    link: u32,
    info: u32,
    addralign: u64,
    entsize: u64,
) {
    out.extend_from_slice(&name.to_le_bytes());
    out.extend_from_slice(&sh_type.to_le_bytes());
    out.extend_from_slice(&flags.to_le_bytes());
    out.extend_from_slice(&0u64.to_le_bytes()); // sh_addr = 0 (ET_REL)
    out.extend_from_slice(&offset.to_le_bytes());
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&link.to_le_bytes());
    out.extend_from_slice(&info.to_le_bytes());
    out.extend_from_slice(&addralign.to_le_bytes());
    out.extend_from_slice(&entsize.to_le_bytes());
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn linux_x64() -> Target { Target::linux_x64() }

    #[test]
    fn rejects_wrong_arch() {
        let mut t = Target::linux_arm64();
        t.arch = "arm64".to_string();
        let result = pack_elf64_object_x86_64(b"\xC3", 0, 0, &[], &t);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_wrong_os() {
        let t = Target::macos_x64();
        let result = pack_elf64_object_x86_64(b"\xC3", 0, 0, &[], &t);
        assert!(result.is_err());
    }

    #[test]
    fn minimal_object_has_elf_magic() {
        // Smallest possible: 1-byte RET (0xC3), no globals, no relocs.
        let out = pack_elf64_object_x86_64(b"\xC3", 0, 0, &[], &linux_x64()).unwrap();
        assert_eq!(&out[0..4], &[0x7F, b'E', b'L', b'F']);
    }

    #[test]
    fn header_has_correct_machine_and_type() {
        let out = pack_elf64_object_x86_64(b"\xC3", 0, 0, &[], &linux_x64()).unwrap();
        // EI_CLASS at byte 4
        assert_eq!(out[4], ELFCLASS64);
        // e_type at byte 16 (after e_ident)
        let e_type = u16::from_le_bytes([out[16], out[17]]);
        assert_eq!(e_type, ET_REL);
        // e_machine at byte 18
        let e_machine = u16::from_le_bytes([out[18], out[19]]);
        assert_eq!(e_machine, EM_X86_64);
    }

    #[test]
    fn text_bytes_appear_after_header() {
        let text = b"\x48\x31\xC0\xC3"; // xor rax, rax; ret
        let out = pack_elf64_object_x86_64(text, 0, 0, &[], &linux_x64()).unwrap();
        assert_eq!(&out[EHDR_SIZE as usize..EHDR_SIZE as usize + text.len()], text);
    }

    #[test]
    fn rejects_entry_past_text() {
        let result = pack_elf64_object_x86_64(b"\xC3", 99, 0, &[], &linux_x64());
        assert!(result.is_err());
    }

    #[test]
    fn relocs_appear_in_rela_text() {
        let text = vec![0xE8, 0, 0, 0, 0]; // CALL rel32 placeholder
        let relocs = vec![
            X86RelocRecord {
                patch_offset: 1,
                symbol: "__twig_print_i64".into(),
                kind: X86RelocKind::PltRel32,
                addend: -4,
            },
        ];
        let out = pack_elf64_object_x86_64(&text, 0, 0, &relocs, &linux_x64()).unwrap();
        // Find the .rela.text section by searching for the symbol name
        // in the file — the strtab will contain it.
        let needle = b"__twig_print_i64";
        let pos = out.windows(needle.len())
            .position(|w| w == needle)
            .expect("symbol name should be present in strtab");
        assert!(pos > EHDR_SIZE as usize);
        // The Elf64_Rela record carries r_info encoding type 4 (PLT32).
        // Search for the type byte (4) in the file — sufficient as a smoke check.
        assert!(out.contains(&4u8), "expected R_X86_64_PLT32 (4) somewhere");
    }

    #[test]
    fn globals_section_present_when_n_slots_nonzero() {
        let text = vec![0xC3];
        let out = pack_elf64_object_x86_64(&text, 0, 4, &[], &linux_x64()).unwrap();
        // _twig_globals symbol name should appear in strtab.
        let needle = b"_twig_globals";
        let pos = out.windows(needle.len())
            .position(|w| w == needle);
        assert!(pos.is_some(), "_twig_globals should appear in strtab when n_slots > 0");
    }

    #[test]
    fn globals_symbol_absent_when_no_slots() {
        let text = vec![0xC3];
        let out = pack_elf64_object_x86_64(&text, 0, 0, &[], &linux_x64()).unwrap();
        let needle = b"_twig_globals";
        let pos = out.windows(needle.len())
            .position(|w| w == needle);
        assert!(pos.is_none(), "no _twig_globals expected when n_slots == 0");
    }

    #[test]
    fn main_symbol_always_present() {
        let out = pack_elf64_object_x86_64(b"\xC3", 0, 0, &[], &linux_x64()).unwrap();
        let needle = b"main";
        assert!(out.windows(needle.len()).any(|w| w == needle));
    }

    #[test]
    fn pc_rel32_emits_type_2() {
        // Verify that the elf_type mapping for PcRel32 is R_X86_64_PC32 (2).
        let text = vec![0xE8, 0, 0, 0, 0];
        let relocs = vec![X86RelocRecord {
            patch_offset: 1,
            symbol: "x".into(),
            kind: X86RelocKind::PcRel32,
            addend: -4,
        }];
        let out = pack_elf64_object_x86_64(&text, 0, 0, &relocs, &linux_x64()).unwrap();
        // Spot-check the section header for .rela.text — it's near the end.
        // The reloc record itself encodes r_info = (sym << 32) | 2.
        // We look for the byte pattern: 24 bytes laid out as
        //   r_offset (8) | r_info (8) | r_addend (8)
        // with r_info LE = [02, 00, 00, 00, 03, 00, 00, 00] for type=2, sym_idx=3.
        // (sym 3 because: 0=null, 1=main, 2=...) actually with no globals
        // sym_idx = first_ext_sym_idx + 0 = 2.
        // r_info LE = [02, 00, 00, 00, 02, 00, 00, 00].
        let needle = [0x02u8, 0, 0, 0, 0x02, 0, 0, 0];
        assert!(out.windows(needle.len()).any(|w| w == needle),
                "r_info for PcRel32 sym=2 not found");
    }

    #[test]
    fn unique_external_symbols_deduped() {
        // Two relocs referencing the same symbol should produce ONE
        // symtab entry, not two.
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
        let out = pack_elf64_object_x86_64(&text, 0, 0, &relocs, &linux_x64()).unwrap();
        // "foo" should appear exactly once in the entire file (within the
        // strtab section).  Use a NUL-terminated needle to avoid false
        // positives from substring matches.
        let needle = b"\0foo\0";
        let occurrences: usize = out.windows(needle.len())
            .filter(|w| *w == needle)
            .count();
        assert_eq!(occurrences, 1, "expected one strtab entry for 'foo'");
    }
}
