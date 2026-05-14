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

// ── Global-variable support (LANG39) + External-branch support (LANG41) ──────

/// ARM64 relocation type codes (from `<mach-o/arm64/reloc.h>`).
///
/// ```text
/// enum reloc_type_arm64 {
///     ARM64_RELOC_UNSIGNED    = 0,  /* for pointers */
///     ARM64_RELOC_SUBTRACTOR  = 1,  /* must precede ARM64_RELOC_UNSIGNED */
///     ARM64_RELOC_BRANCH26    = 2,  /* B/BL with 26-bit displacement */
///     ARM64_RELOC_PAGE21      = 3,  /* pc-rel distance to page of target */
///     ARM64_RELOC_PAGEOFF12   = 4,  /* offset within page, scaled by r_length */
///     …
/// };
/// ```
const ARM64_RELOC_BRANCH26:  u32 = 2; // B/BL instruction — new in LANG41
const ARM64_RELOC_PAGE21:    u32 = 3; // ADRP instruction
const ARM64_RELOC_PAGEOFF12: u32 = 4; // ADD/LDR/STR instruction

// nlist_64 n_type constants for undefined external symbols (LANG41).
//
// An undefined external symbol (`N_UNDF | N_EXT`) in the symbol table tells
// the system linker (`ld`) that this compilation unit requires the symbol to
// be defined by another object file or static archive.  `ld` resolves it from
// the libraries and archives passed on its command line (e.g. the Twig AOT
// runtime archive that provides `__twig_print_i64`).
//
// `N_UNDF = 0x00` is the "undefined" type; OR'ing with `N_EXT = 0x01` marks
// the symbol as external (linkage-visible).  For undefined symbols n_sect = 0
// (NO_SECT) and n_value = 0.
const N_UNDF: u8 = 0x00;

/// Byte offsets within the `__text` section of one `ADRP + ADD` instruction
/// pair that must be patched by the system linker to address `_twig_globals`.
///
/// These correspond directly to [`aarch64_backend::GlobalWordReloc`] after
/// converting word indices to byte offsets.
///
/// The Mach-O packager emits two relocation records per [`GlobalByteReloc`]:
///
/// 1. `ARM64_RELOC_PAGE21` at `adrp_byte_offset` — patches the 21-bit page
///    offset in the `ADRP X1, #0` instruction.
/// 2. `ARM64_RELOC_PAGEOFF12` at `add_byte_offset` — patches the 12-bit
///    page-relative offset in the `ADD X1, X1, #0` instruction.
///
/// Both records reference the `_twig_globals` symbol (symbol table index 1).
#[derive(Debug, Clone, Copy)]
pub struct GlobalByteReloc {
    /// Byte offset in the linked `__text` section of the `ADRP` instruction.
    pub adrp_byte_offset: u32,
    /// Byte offset in the linked `__text` section of the `ADD` instruction.
    pub add_byte_offset:  u32,
}

// Header constant with two sections (for pack_object_with_globals).
//
// LC_SEGMENT_64 command with 2 sections = 72 + 2*80 = 232
const LC_SEGMENT_TWO_SECTS: usize = LC_SEGMENT_SIZE + 2 * SECTION_SIZE;
const HEADER_TOTAL_WITH_DATA: usize = MACH_HEADER_SIZE
    + LC_SEGMENT_TWO_SECTS
    + LC_BUILD_VERSION_SIZE
    + LC_SYMTAB_SIZE; // = 312

/// Like [`pack_object`] but also emits a `__DATA/__data` section that holds
/// the Twig program's global variable slots, plus `ARM64_RELOC_PAGE21` /
/// `ARM64_RELOC_PAGEOFF12` relocation records so the system linker can patch
/// the `ADRP + ADD` address-materialisation pairs in the code.
///
/// # Parameters
///
/// - `text_bytes` — ARM64 machine code for the `__text` section.
/// - `entry_point` — byte offset of `_main` within `text_bytes`.
/// - `globals_n_slots` — number of 8-byte global variable slots.  The
///   `__data` section will be `globals_n_slots * 8` bytes of zeroes.
///   Pass `0` if there are no global accesses (the function degenerates to
///   [`pack_object`] but with two sections and two symbols).
/// - `text_relocs` — one [`GlobalByteReloc`] per `global_load`/`global_store`
///   instruction across all compiled functions.
/// - `target` — must be `macos` + `arm64` (x86_64 global support is NYI).
///
/// # Object-file layout
///
/// ```text
/// 0              │ 312     │ Headers (mach_header_64 + LC_SEGMENT_64×2 + LC_BUILD_VERSION + LC_SYMTAB)
/// 312            │ N       │ __text bytes
/// 312+N          │ M       │ __data bytes (zero-initialised globals, M = globals_n_slots * 8)
/// 312+N+M        │ 2*R*8   │ text relocation records (2 per global access = 2 × R × 8 bytes)
/// 312+N+M+2*R*8  │ 32      │ 2 nlist_64 entries: _main (sect 1) + _twig_globals (sect 2)
/// …              │ S       │ string table: "\0_main\0_twig_globals\0"
/// ```
///
/// where `R = text_relocs.len()`.
pub fn pack_object_with_globals(
    text_bytes: &[u8],
    entry_point: usize,
    globals_n_slots: usize,
    text_relocs: &[GlobalByteReloc],
    target: &Target,
) -> Result<Vec<u8>, PackagerError> {
    if target.os != "macos" {
        return Err(PackagerError::UnsupportedTarget(format!(
            "pack_object_with_globals expects os=macos, got {:?}", target.os
        )));
    }
    if target.arch != "arm64" {
        return Err(PackagerError::UnsupportedTarget(format!(
            "pack_object_with_globals: ARM64 global relocations only; got arch {:?}", target.arch
        )));
    }
    let (cputype, cpusubtype) = cpu_type(target)?;

    let code_len   = text_bytes.len() as u64;
    let data_len   = (globals_n_slots * 8) as u64;
    let n_relocs   = text_relocs.len() as u32;     // each becomes 2 reloc records
    let reloc_bytes = (n_relocs as u64) * 2 * 8;   // 8 bytes per relocation_info
    let entry_off  = entry_point as u64;

    if entry_off > code_len {
        return Err(PackagerError::UnsupportedTarget(format!(
            "pack_object_with_globals: entry_point {entry_off} exceeds text length {code_len}"
        )));
    }

    // ── String table ─────────────────────────────────────────────────────────
    //
    // Layout: "\0_main\0_twig_globals\0"
    //                                 ^-- symbol 1 n_strx = 1 (_main)
    //                                             ^-- symbol 2 n_strx = 7 (_twig_globals)
    const GLOBALS_SYMBOL: &str = "_twig_globals";
    let strtab: Vec<u8> = {
        let mut s = Vec::new();
        s.push(0u8);                                // leading NUL (reserved)
        s.extend_from_slice(ENTRY_SYMBOL.as_bytes()); // "_main"
        s.push(0u8);
        s.extend_from_slice(GLOBALS_SYMBOL.as_bytes()); // "_twig_globals"
        s.push(0u8);
        s
    };
    // n_strx for _twig_globals = 1 (skip leading NUL) + len("_main") + 1 (NUL)
    let twig_globals_strx: u32 = 1 + ENTRY_SYMBOL.len() as u32 + 1;

    // ── File-offset calculations ──────────────────────────────────────────────
    let text_file_off  = HEADER_TOTAL_WITH_DATA as u64;
    let data_file_off  = text_file_off + code_len;
    let reloc_file_off = data_file_off + data_len;
    let symtab_off     = reloc_file_off + reloc_bytes;
    let strtab_off     = symtab_off + 2 * NLIST_64_SIZE as u64; // 2 symbols

    let total_size: usize = strtab_off as usize + strtab.len();

    // ── sizeofcmds ───────────────────────────────────────────────────────────
    let sizeofcmds: u32 = (LC_SEGMENT_TWO_SECTS + LC_BUILD_VERSION_SIZE + LC_SYMTAB_SIZE) as u32;

    // ── Checked u64 → u32 conversions for Mach-O 32-bit file-offset fields ──
    //
    // All Mach-O section and symtab file-offset fields are 32-bit.  For normal
    // AOT object files these offsets will never approach 4 GiB, but we verify
    // that explicitly so a pathological input (e.g. a module with >512 MB of
    // machine code) produces a clear error rather than a silently malformed
    // object file with truncated offsets.
    //
    // Helper closure: try_into() with a meaningful PackagerError on overflow.
    macro_rules! off32 {
        ($val:expr, $name:expr) => {
            u32::try_from($val).map_err(|_| PackagerError::UnsupportedTarget(format!(
                "pack_object_with_globals: {} file offset {} exceeds 4 GiB; \
                 object files do not support this",
                $name, $val
            )))?
        };
    }
    let text_file_off_u32  = off32!(text_file_off,  "text");
    let reloc_file_off_u32 = off32!(reloc_file_off, "reloc");
    let data_file_off_u32  = off32!(data_file_off,  "data");
    let symtab_off_u32     = off32!(symtab_off,     "symtab");
    let strtab_off_u32     = off32!(strtab_off,     "strtab");

    let mut out: Vec<u8> = Vec::with_capacity(total_size);

    // ── mach_header_64 ───────────────────────────────────────────────────────
    out.extend_from_slice(&MH_MAGIC_64.to_le_bytes());
    out.extend_from_slice(&cputype.to_le_bytes());
    out.extend_from_slice(&cpusubtype.to_le_bytes());
    out.extend_from_slice(&MH_OBJECT.to_le_bytes());
    out.extend_from_slice(&3u32.to_le_bytes());          // ncmds = 3
    out.extend_from_slice(&sizeofcmds.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());          // flags
    out.extend_from_slice(&0u32.to_le_bytes());          // reserved
    debug_assert_eq!(out.len(), MACH_HEADER_SIZE);

    // ── LC_SEGMENT_64 (two sections: __text + __data) ─────────────────────
    //
    // In MH_OBJECT files the segment command has an empty segname and covers
    // all sections.  The vmsize spans both sections back-to-back.
    let segment_vmsize = code_len + data_len;
    out.extend_from_slice(&LC_SEGMENT_64.to_le_bytes());
    out.extend_from_slice(&(LC_SEGMENT_TWO_SECTS as u32).to_le_bytes()); // cmdsize
    write_name16(&mut out, b"");                  // segname = empty
    out.extend_from_slice(&0u64.to_le_bytes());   // vmaddr = 0
    out.extend_from_slice(&segment_vmsize.to_le_bytes()); // vmsize
    out.extend_from_slice(&text_file_off.to_le_bytes());  // fileoff = first section offset
    out.extend_from_slice(&segment_vmsize.to_le_bytes()); // filesize
    out.extend_from_slice(&7u32.to_le_bytes());   // maxprot  = rwx
    out.extend_from_slice(&7u32.to_le_bytes());   // initprot = rwx
    out.extend_from_slice(&2u32.to_le_bytes());   // nsects = 2
    out.extend_from_slice(&0u32.to_le_bytes());   // flags

    // ── section_64: __text / __TEXT ─────────────────────────────────────────
    //
    // reloff points to the relocation records that follow the data section.
    write_name16(&mut out, b"__text");
    write_name16(&mut out, b"__TEXT");
    out.extend_from_slice(&0u64.to_le_bytes());          // addr = 0 (relocatable)
    out.extend_from_slice(&code_len.to_le_bytes());       // size
    out.extend_from_slice(&text_file_off_u32.to_le_bytes()); // offset
    out.extend_from_slice(&4u32.to_le_bytes());           // align = 2^4 = 16
    // reloff = file offset of our relocation records
    out.extend_from_slice(&reloc_file_off_u32.to_le_bytes());
    // nreloc = 2 records per GlobalByteReloc (PAGE21 + PAGEOFF12)
    out.extend_from_slice(&(n_relocs * 2).to_le_bytes());
    out.extend_from_slice(&SECTION_FLAGS_CODE.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());           // reserved1
    out.extend_from_slice(&0u32.to_le_bytes());           // reserved2
    out.extend_from_slice(&0u32.to_le_bytes());           // reserved3

    // ── section_64: __data / __DATA ─────────────────────────────────────────
    //
    // Zero-initialised mutable data holding global variable slots.
    // No relocations in the data section itself.
    write_name16(&mut out, b"__data");
    write_name16(&mut out, b"__DATA");
    out.extend_from_slice(&code_len.to_le_bytes());  // addr = immediately after __text
    out.extend_from_slice(&data_len.to_le_bytes());  // size = n_slots * 8
    out.extend_from_slice(&data_file_off_u32.to_le_bytes()); // offset
    out.extend_from_slice(&3u32.to_le_bytes());       // align = 2^3 = 8 (64-bit slots)
    out.extend_from_slice(&0u32.to_le_bytes());       // reloff = 0 (no data relocs)
    out.extend_from_slice(&0u32.to_le_bytes());       // nreloc = 0
    out.extend_from_slice(&0u32.to_le_bytes());       // flags = S_REGULAR (plain data)
    out.extend_from_slice(&0u32.to_le_bytes());       // reserved1
    out.extend_from_slice(&0u32.to_le_bytes());       // reserved2
    out.extend_from_slice(&0u32.to_le_bytes());       // reserved3

    debug_assert_eq!(out.len(), MACH_HEADER_SIZE + LC_SEGMENT_TWO_SECTS);

    // ── LC_BUILD_VERSION ─────────────────────────────────────────────────────
    out.extend_from_slice(&LC_BUILD_VERSION.to_le_bytes());
    out.extend_from_slice(&(LC_BUILD_VERSION_SIZE as u32).to_le_bytes());
    out.extend_from_slice(&PLATFORM_MACOS.to_le_bytes());
    out.extend_from_slice(&MIN_OS_VERSION.to_le_bytes());
    out.extend_from_slice(&SDK_VERSION.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());       // ntools

    // ── LC_SYMTAB ────────────────────────────────────────────────────────────
    out.extend_from_slice(&LC_SYMTAB.to_le_bytes());
    out.extend_from_slice(&(LC_SYMTAB_SIZE as u32).to_le_bytes());
    out.extend_from_slice(&symtab_off_u32.to_le_bytes());          // symoff
    out.extend_from_slice(&2u32.to_le_bytes());                   // nsyms = 2
    out.extend_from_slice(&strtab_off_u32.to_le_bytes());          // stroff
    out.extend_from_slice(&(strtab.len() as u32).to_le_bytes());  // strsize

    debug_assert_eq!(out.len(), HEADER_TOTAL_WITH_DATA);

    // ── __text bytes ─────────────────────────────────────────────────────────
    out.extend_from_slice(text_bytes);

    // ── __data bytes (zero-initialised) ──────────────────────────────────────
    let data_start = out.len();
    out.resize(data_start + data_len as usize, 0u8);

    // ── ARM64 relocation records ──────────────────────────────────────────────
    //
    // Each GlobalByteReloc produces two relocation_info records (8 bytes each):
    //
    //   ARM64_RELOC_PAGE21 on the ADRP instruction:
    //     r_address    = adrp_byte_offset
    //     r_symbolnum  = 1 (index of _twig_globals in symtab)
    //     r_pcrel      = 1
    //     r_length     = 2  (log2(4) = 2 for 4-byte instruction)
    //     r_extern     = 1  (symbol-relative, not section-relative)
    //     r_type       = ARM64_RELOC_PAGE21 (3)
    //
    //   ARM64_RELOC_PAGEOFF12 on the ADD instruction:
    //     r_address    = add_byte_offset
    //     r_symbolnum  = 1
    //     r_pcrel      = 0
    //     r_length     = 2
    //     r_extern     = 1
    //     r_type       = ARM64_RELOC_PAGEOFF12 (4)
    //
    // The packed u32 (second field):
    //   bits  0-23: r_symbolnum
    //   bit     24: r_pcrel
    //   bits 25-26: r_length
    //   bit     27: r_extern
    //   bits 28-31: r_type
    //
    // SYMBOL_IDX = 1 (_twig_globals is the second symbol in the symtab).
    const SYMBOL_IDX: u32 = 1;

    // PAGE21:    symbolnum=1, pcrel=1, length=2, extern=1, type=3
    //   packed = 1 | (1<<24) | (2<<25) | (1<<27) | (3<<28) = 0x3D000001
    let page21_info: u32 =
        SYMBOL_IDX
        | (1u32 << 24)                      // r_pcrel
        | (2u32 << 25)                      // r_length = 2
        | (1u32 << 27)                      // r_extern
        | (ARM64_RELOC_PAGE21 << 28);       // r_type

    // PAGEOFF12: symbolnum=1, pcrel=0, length=2, extern=1, type=4
    //   packed = 1 | (0<<24) | (2<<25) | (1<<27) | (4<<28) = 0x4C000001
    let pageoff12_info: u32 =
        SYMBOL_IDX
        | (0u32 << 24)                      // r_pcrel = 0
        | (2u32 << 25)                      // r_length = 2
        | (1u32 << 27)                      // r_extern
        | (ARM64_RELOC_PAGEOFF12 << 28);    // r_type

    for reloc in text_relocs {
        // ARM64_RELOC_PAGE21 on ADRP
        out.extend_from_slice(&(reloc.adrp_byte_offset as i32).to_le_bytes());
        out.extend_from_slice(&page21_info.to_le_bytes());

        // ARM64_RELOC_PAGEOFF12 on ADD
        out.extend_from_slice(&(reloc.add_byte_offset as i32).to_le_bytes());
        out.extend_from_slice(&pageoff12_info.to_le_bytes());
    }

    // ── Symbol table — _main (index 0) + _twig_globals (index 1) ─────────────
    //
    // _main: in __text (section 1), at entry_point offset.
    out.extend_from_slice(&1u32.to_le_bytes());          // n_strx (skip leading NUL)
    out.push(N_SECT | N_EXT);                            // n_type
    out.push(1u8);                                       // n_sect = 1 (__text)
    out.extend_from_slice(&0u16.to_le_bytes());          // n_desc
    out.extend_from_slice(&entry_off.to_le_bytes());     // n_value (byte offset)

    // _twig_globals: in __data (section 2), at offset 0 within that section.
    out.extend_from_slice(&twig_globals_strx.to_le_bytes()); // n_strx
    out.push(N_SECT | N_EXT);                           // n_type
    out.push(2u8);                                       // n_sect = 2 (__data)
    out.extend_from_slice(&0u16.to_le_bytes());          // n_desc
    out.extend_from_slice(&code_len.to_le_bytes());      // n_value = VM addr of __data start

    // ── String table ─────────────────────────────────────────────────────────
    out.extend_from_slice(&strtab);

    debug_assert_eq!(out.len(), total_size, "layout mismatch: expected {total_size} got {}", out.len());
    Ok(out)
}

// ── External-branch relocations (LANG41) ─────────────────────────────────────

/// An unresolved external branch: a `BL` instruction in `__text` that targets
/// a symbol defined outside this compilation unit (e.g. in the Twig AOT
/// runtime library).
///
/// `twig-aot`'s two-pass linker collects these from compiled functions that
/// reference symbols not present in the module (e.g. `__twig_print_i64` from
/// the `io_out` CIR opcode).  The packager emits:
///
/// 1. An `N_UNDF | N_EXT` symbol-table entry for each unique symbol.
/// 2. An `ARM64_RELOC_BRANCH26` (r_extern=1) relocation record for each `BL`
///    instruction, so the system linker can patch the 26-bit immediate.
///
/// The `BL` instruction itself is left as `0x94000000` (offset = 0) by the
/// ARM64 backend's `bl_external` method — the linker patches it at final
/// link time.
#[derive(Debug, Clone)]
pub struct ExternBranchReloc {
    /// Byte offset of the `BL` instruction within the linked `__text` section.
    pub byte_offset: u32,
    /// External symbol name (e.g. `"__twig_print_i64"`).
    pub symbol: String,
}

/// Like [`pack_object_with_globals`] but also handles unresolved external
/// branch relocations (LANG41).
///
/// When a Twig function calls a runtime helper (e.g. `__twig_print_i64` for
/// the `io_out` / `print` builtin), the ARM64 backend emits a `BL` with a
/// placeholder offset and records an [`ExternBranchReloc`].  This function
/// packages those as:
///
/// - `N_UNDF | N_EXT` symbol-table entries (one per unique external symbol).
/// - `ARM64_RELOC_BRANCH26` relocation records (one per `BL` site) so the
///   system linker can patch the 26-bit PC-relative offset.
///
/// The runtime archive (providing the external symbols) must be passed to the
/// system linker separately — `twig-aot` embeds the archive bytes and writes
/// them to a temp file that is added to the `ld` command line.
///
/// # Parameters
///
/// - `text_bytes` — ARM64 machine code for `__text`.
/// - `entry_point` — byte offset of `_main` within `text_bytes`.
/// - `globals_n_slots` — number of 8-byte global variable slots (may be 0).
/// - `global_relocs` — `ADRP + ADD` relocation pairs for `_twig_globals`.
/// - `extern_relocs` — one [`ExternBranchReloc`] per external `BL` site.
/// - `target` — must be `macos` + `arm64`.
///
/// # Object-file layout
///
/// ```text
/// 0              │ 312       │ Headers (mach_header_64 + 2×LC_SEGMENT_64 + LC_BUILD_VERSION + LC_SYMTAB)
/// 312            │ N         │ __text bytes
/// 312+N          │ M         │ __data bytes (zero-init, M = globals_n_slots × 8; may be 0)
/// 312+N+M        │ (E+2G)×8  │ relocation records:
///                │           │   E × ARM64_RELOC_BRANCH26 (extern BL sites)
///                │           │   2G × ARM64_RELOC_PAGE21 / PAGEOFF12 (global ADRP+ADD pairs)
/// …              │ (2+U)×16  │ symbol table: _main + _twig_globals + U undefined externals
/// …              │ S         │ string table
/// ```
///
/// where E = extern_relocs.len(), G = global_relocs.len(),
/// U = number of unique symbols in extern_relocs.
pub fn pack_object_with_globals_and_externals(
    text_bytes: &[u8],
    entry_point: usize,
    globals_n_slots: usize,
    global_relocs: &[GlobalByteReloc],
    extern_relocs: &[ExternBranchReloc],
    target: &Target,
) -> Result<Vec<u8>, PackagerError> {
    if target.os != "macos" {
        return Err(PackagerError::UnsupportedTarget(format!(
            "pack_object_with_globals_and_externals expects os=macos, got {:?}",
            target.os
        )));
    }
    if target.arch != "arm64" {
        return Err(PackagerError::UnsupportedTarget(format!(
            "pack_object_with_globals_and_externals: ARM64 only; got arch {:?}",
            target.arch
        )));
    }
    let (cputype, cpusubtype) = cpu_type(target)?;

    let code_len  = text_bytes.len() as u64;
    let data_len  = (globals_n_slots * 8) as u64;
    let entry_off = entry_point as u64;

    if entry_off > code_len {
        return Err(PackagerError::UnsupportedTarget(format!(
            "pack_object_with_globals_and_externals: entry_point {entry_off} exceeds text length {code_len}"
        )));
    }

    // ── Unique external symbols (preserve order of first appearance) ──────────
    //
    // The symbol-table index of each unique extern is (2 + position_in_vec):
    //   index 0 = _main
    //   index 1 = _twig_globals
    //   index 2 = first unique extern
    //   index 3 = second unique extern
    //   …
    //
    // Deduplication uses a HashMap for O(n) total work (Vec::contains is O(n²)
    // and would enable a DoS via a large number of unique symbol names in a
    // caller-controlled ExternBranchReloc slice).
    let mut unique_ext_syms: Vec<&str> = Vec::new();
    {
        let mut seen: std::collections::HashMap<&str, ()> = std::collections::HashMap::new();
        for er in extern_relocs {
            if seen.insert(er.symbol.as_str(), ()).is_none() {
                unique_ext_syms.push(&er.symbol);
            }
        }
    }
    let n_ext_syms = unique_ext_syms.len();

    // ── Relocation record counts ──────────────────────────────────────────────
    //
    // Global accesses produce 2 records each (PAGE21 + PAGEOFF12).
    // External BL sites produce 1 record each (BRANCH26).
    let n_global_pairs  = global_relocs.len() as u32;
    let n_extern_bls    = extern_relocs.len() as u32;
    let total_reloc_recs = n_global_pairs * 2 + n_extern_bls;
    let reloc_bytes     = (total_reloc_recs as u64) * 8; // 8 bytes per relocation_info

    // ── Symbol counts ─────────────────────────────────────────────────────────
    let n_symbols = 2u32 + n_ext_syms as u32; // _main + _twig_globals + externals

    // ── String table ─────────────────────────────────────────────────────────
    //
    // Layout: "\0_main\0_twig_globals\0<ext0>\0<ext1>\0…"
    const GLOBALS_SYMBOL: &str = "_twig_globals";
    let mut strtab: Vec<u8> = Vec::new();
    strtab.push(0u8);                                      // leading NUL (sentinel)
    let main_strx      = strtab.len() as u32;              // = 1
    strtab.extend_from_slice(ENTRY_SYMBOL.as_bytes());     // "_main"
    strtab.push(0u8);
    let globals_strx   = strtab.len() as u32;             // = 1 + 5 + 1 = 7
    strtab.extend_from_slice(GLOBALS_SYMBOL.as_bytes());  // "_twig_globals"
    strtab.push(0u8);
    let mut ext_strx: Vec<u32> = Vec::with_capacity(n_ext_syms);
    for sym in &unique_ext_syms {
        ext_strx.push(strtab.len() as u32);
        strtab.extend_from_slice(sym.as_bytes());
        strtab.push(0u8);
    }

    // ── File-offset calculations ──────────────────────────────────────────────
    let text_file_off  = HEADER_TOTAL_WITH_DATA as u64;
    let data_file_off  = text_file_off + code_len;
    let reloc_file_off = data_file_off + data_len;
    let symtab_off     = reloc_file_off + reloc_bytes;
    let strtab_off     = symtab_off + (n_symbols as u64) * NLIST_64_SIZE as u64;
    let total_size: usize = strtab_off as usize + strtab.len();

    // Checked u32 conversions for Mach-O file-offset fields.
    macro_rules! off32 {
        ($val:expr, $name:expr) => {
            u32::try_from($val).map_err(|_| PackagerError::UnsupportedTarget(format!(
                "pack_object_with_globals_and_externals: {} offset {} exceeds 4 GiB",
                $name, $val
            )))?
        };
    }
    let text_file_off_u32  = off32!(text_file_off,  "text");
    let data_file_off_u32  = off32!(data_file_off,  "data");
    let reloc_file_off_u32 = off32!(reloc_file_off, "reloc");
    let symtab_off_u32     = off32!(symtab_off,     "symtab");
    let strtab_off_u32     = off32!(strtab_off,     "strtab");

    let sizeofcmds: u32 = (LC_SEGMENT_TWO_SECTS + LC_BUILD_VERSION_SIZE + LC_SYMTAB_SIZE) as u32;

    let mut out: Vec<u8> = Vec::with_capacity(total_size);

    // ── mach_header_64 ───────────────────────────────────────────────────────
    out.extend_from_slice(&MH_MAGIC_64.to_le_bytes());
    out.extend_from_slice(&cputype.to_le_bytes());
    out.extend_from_slice(&cpusubtype.to_le_bytes());
    out.extend_from_slice(&MH_OBJECT.to_le_bytes());
    out.extend_from_slice(&3u32.to_le_bytes());         // ncmds = 3
    out.extend_from_slice(&sizeofcmds.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());         // flags
    out.extend_from_slice(&0u32.to_le_bytes());         // reserved
    debug_assert_eq!(out.len(), MACH_HEADER_SIZE);

    // ── LC_SEGMENT_64 (two sections: __text + __data) ─────────────────────
    let segment_vmsize = code_len + data_len;
    out.extend_from_slice(&LC_SEGMENT_64.to_le_bytes());
    out.extend_from_slice(&(LC_SEGMENT_TWO_SECTS as u32).to_le_bytes());
    write_name16(&mut out, b"");                 // segname = empty
    out.extend_from_slice(&0u64.to_le_bytes()); // vmaddr = 0
    out.extend_from_slice(&segment_vmsize.to_le_bytes());
    out.extend_from_slice(&text_file_off.to_le_bytes());
    out.extend_from_slice(&segment_vmsize.to_le_bytes());
    out.extend_from_slice(&7u32.to_le_bytes()); // maxprot  = rwx
    out.extend_from_slice(&7u32.to_le_bytes()); // initprot = rwx
    out.extend_from_slice(&2u32.to_le_bytes()); // nsects = 2
    out.extend_from_slice(&0u32.to_le_bytes()); // flags

    // ── section_64: __text / __TEXT ─────────────────────────────────────────
    write_name16(&mut out, b"__text");
    write_name16(&mut out, b"__TEXT");
    out.extend_from_slice(&0u64.to_le_bytes());                      // addr = 0
    out.extend_from_slice(&code_len.to_le_bytes());                  // size
    out.extend_from_slice(&text_file_off_u32.to_le_bytes());         // offset
    out.extend_from_slice(&4u32.to_le_bytes());                      // align = 2^4 = 16
    out.extend_from_slice(&reloc_file_off_u32.to_le_bytes());        // reloff
    out.extend_from_slice(&total_reloc_recs.to_le_bytes());          // nreloc
    out.extend_from_slice(&SECTION_FLAGS_CODE.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());                      // reserved1
    out.extend_from_slice(&0u32.to_le_bytes());                      // reserved2
    out.extend_from_slice(&0u32.to_le_bytes());                      // reserved3

    // ── section_64: __data / __DATA ─────────────────────────────────────────
    write_name16(&mut out, b"__data");
    write_name16(&mut out, b"__DATA");
    out.extend_from_slice(&code_len.to_le_bytes());       // addr = immediately after __text
    out.extend_from_slice(&data_len.to_le_bytes());       // size (0 if no globals)
    out.extend_from_slice(&data_file_off_u32.to_le_bytes()); // offset
    out.extend_from_slice(&3u32.to_le_bytes());           // align = 2^3 = 8
    out.extend_from_slice(&0u32.to_le_bytes());           // reloff = 0
    out.extend_from_slice(&0u32.to_le_bytes());           // nreloc = 0
    out.extend_from_slice(&0u32.to_le_bytes());           // flags = S_REGULAR
    out.extend_from_slice(&0u32.to_le_bytes());           // reserved1
    out.extend_from_slice(&0u32.to_le_bytes());           // reserved2
    out.extend_from_slice(&0u32.to_le_bytes());           // reserved3

    debug_assert_eq!(out.len(), MACH_HEADER_SIZE + LC_SEGMENT_TWO_SECTS);

    // ── LC_BUILD_VERSION ─────────────────────────────────────────────────────
    out.extend_from_slice(&LC_BUILD_VERSION.to_le_bytes());
    out.extend_from_slice(&(LC_BUILD_VERSION_SIZE as u32).to_le_bytes());
    out.extend_from_slice(&PLATFORM_MACOS.to_le_bytes());
    out.extend_from_slice(&MIN_OS_VERSION.to_le_bytes());
    out.extend_from_slice(&SDK_VERSION.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // ntools

    // ── LC_SYMTAB ────────────────────────────────────────────────────────────
    out.extend_from_slice(&LC_SYMTAB.to_le_bytes());
    out.extend_from_slice(&(LC_SYMTAB_SIZE as u32).to_le_bytes());
    out.extend_from_slice(&symtab_off_u32.to_le_bytes());
    out.extend_from_slice(&n_symbols.to_le_bytes());
    out.extend_from_slice(&strtab_off_u32.to_le_bytes());
    out.extend_from_slice(&(strtab.len() as u32).to_le_bytes());

    debug_assert_eq!(out.len(), HEADER_TOTAL_WITH_DATA);

    // ── __text bytes ─────────────────────────────────────────────────────────
    out.extend_from_slice(text_bytes);

    // ── __data bytes (zero-initialised) ──────────────────────────────────────
    let data_start = out.len();
    out.resize(data_start + data_len as usize, 0u8);

    // ── Relocation records ────────────────────────────────────────────────────
    //
    // The packed `r_info` u32 layout (ARM64 Mach-O):
    //   bits  0-23: r_symbolnum (24-bit symbol-table index)
    //   bit     24: r_pcrel     (1 = PC-relative, 0 = absolute)
    //   bits 25-26: r_length    (log2 of instruction byte width: 2 → 4 bytes)
    //   bit     27: r_extern    (1 = symbol-table index, 0 = section index)
    //   bits 28-31: r_type      (ARM64 relocation type)

    // ── BRANCH26 records (one per extern BL) — emitted first ──────────────
    //
    // ARM64_RELOC_BRANCH26: r_pcrel=1, r_length=2, r_extern=1, r_type=2.
    // r_symbolnum = 2 + position in unique_ext_syms (after _main and _twig_globals).
    //
    // The `BL` instruction placeholder (`0x94000000`) already has the correct
    // opcode bits; the linker patches only the 26-bit immediate field.
    for er in extern_relocs {
        let sym_idx = unique_ext_syms
            .iter()
            .position(|&s| s == er.symbol)
            .expect("all extern symbols were collected above") as u32
            + 2; // +2 to skip _main (0) and _twig_globals (1)
        let branch26_info: u32 = sym_idx
            | (1u32 << 24)                         // r_pcrel = 1
            | (2u32 << 25)                         // r_length = 2
            | (1u32 << 27)                         // r_extern = 1
            | (ARM64_RELOC_BRANCH26 << 28);        // r_type = 2
        out.extend_from_slice(&(er.byte_offset as i32).to_le_bytes());
        out.extend_from_slice(&branch26_info.to_le_bytes());
    }

    // ── PAGE21 + PAGEOFF12 records (2 per global access) — emitted after ──
    //
    // _twig_globals is always symbol index 1.
    const GLOBALS_SYM_IDX: u32 = 1;

    // PAGE21 (ADRP): r_pcrel=1, r_length=2, r_extern=1, r_type=3.
    let page21_info: u32 = GLOBALS_SYM_IDX
        | (1u32 << 24)                             // r_pcrel = 1
        | (2u32 << 25)                             // r_length = 2
        | (1u32 << 27)                             // r_extern = 1
        | (ARM64_RELOC_PAGE21 << 28);              // r_type = 3

    // PAGEOFF12 (ADD): r_pcrel=0, r_length=2, r_extern=1, r_type=4.
    let pageoff12_info: u32 = GLOBALS_SYM_IDX
        | (0u32 << 24)                             // r_pcrel = 0
        | (2u32 << 25)                             // r_length = 2
        | (1u32 << 27)                             // r_extern = 1
        | (ARM64_RELOC_PAGEOFF12 << 28);           // r_type = 4

    for reloc in global_relocs {
        // ARM64_RELOC_PAGE21 on ADRP
        out.extend_from_slice(&(reloc.adrp_byte_offset as i32).to_le_bytes());
        out.extend_from_slice(&page21_info.to_le_bytes());
        // ARM64_RELOC_PAGEOFF12 on ADD
        out.extend_from_slice(&(reloc.add_byte_offset as i32).to_le_bytes());
        out.extend_from_slice(&pageoff12_info.to_le_bytes());
    }

    // ── Symbol table ─────────────────────────────────────────────────────────

    // index 0: _main — defined in __text (section 1), at entry_off.
    out.extend_from_slice(&main_strx.to_le_bytes());
    out.push(N_SECT | N_EXT);
    out.push(1u8);                                     // n_sect = 1 (__text)
    out.extend_from_slice(&0u16.to_le_bytes());        // n_desc
    out.extend_from_slice(&entry_off.to_le_bytes());   // n_value = byte offset

    // index 1: _twig_globals — defined in __data (section 2), at VM start of data.
    out.extend_from_slice(&globals_strx.to_le_bytes());
    out.push(N_SECT | N_EXT);
    out.push(2u8);                                     // n_sect = 2 (__data)
    out.extend_from_slice(&0u16.to_le_bytes());        // n_desc
    out.extend_from_slice(&code_len.to_le_bytes());    // n_value = VM addr of __data

    // index 2, 3, …: unique external symbols — undefined externals.
    //
    // N_UNDF | N_EXT = 0x01.  n_sect = 0 (NO_SECT).  n_value = 0.
    // The system linker resolves these from the runtime archive or dylibs.
    for (i, sym) in unique_ext_syms.iter().enumerate() {
        let _ = sym; // name is in the string table via ext_strx
        out.extend_from_slice(&ext_strx[i].to_le_bytes());
        out.push(N_UNDF | N_EXT);
        out.push(0u8);                                 // n_sect = 0 (NO_SECT)
        out.extend_from_slice(&0u16.to_le_bytes());    // n_desc
        out.extend_from_slice(&0u64.to_le_bytes());    // n_value = 0 (undefined)
    }

    // ── String table ─────────────────────────────────────────────────────────
    out.extend_from_slice(&strtab);

    debug_assert_eq!(
        out.len(), total_size,
        "layout mismatch: expected {total_size} got {}",
        out.len()
    );
    Ok(out)
}

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

    // ── pack_object_with_globals tests (LANG39) ───────────────────────────────

    fn arm64_globals_bytes(code: Vec<u8>, n_slots: usize, relocs: &[GlobalByteReloc]) -> Vec<u8> {
        pack_object_with_globals(&code, 0, n_slots, relocs, &Target::macos_arm64()).unwrap()
    }

    #[test]
    fn globals_produces_macho_magic() {
        let bytes = arm64_globals_bytes(vec![0x00; 4], 2, &[]);
        assert_eq!(&bytes[0..4], &[0xCF, 0xFA, 0xED, 0xFE]);
    }

    #[test]
    fn globals_filetype_is_mh_object() {
        let bytes = arm64_globals_bytes(vec![0x00; 4], 1, &[]);
        let filetype = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
        assert_eq!(filetype, 1u32, "MH_OBJECT");
    }

    #[test]
    fn globals_ncmds_is_three() {
        let bytes = arm64_globals_bytes(vec![0x00; 4], 1, &[]);
        let ncmds = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
        assert_eq!(ncmds, 3);
    }

    #[test]
    fn globals_header_is_312_bytes() {
        // With 2 sections, HEADER_TOTAL_WITH_DATA = 312.
        let code = vec![0x00u8; 8];
        let bytes = arm64_globals_bytes(code, 3, &[]);
        // text starts at byte 312.
        assert_eq!(&bytes[312..320], &[0x00u8; 8]);
    }

    #[test]
    fn globals_data_section_is_zeroed() {
        let code = vec![0x42u8; 4];
        let n_slots = 4;
        let bytes = arm64_globals_bytes(code.clone(), n_slots, &[]);
        // __data starts at 312 + code_len = 312 + 4 = 316
        let data_start = 312 + 4;
        let data = &bytes[data_start..data_start + n_slots * 8];
        assert_eq!(data, &vec![0u8; n_slots * 8][..], "__data must be zero-init");
    }

    #[test]
    fn globals_two_relocs_per_access() {
        let code = vec![0x00u8; 16];
        let relocs = vec![
            GlobalByteReloc { adrp_byte_offset: 0, add_byte_offset: 4 },
        ];
        let n_slots = 1;
        let bytes = arm64_globals_bytes(code, n_slots, &relocs);
        // relocation records are at 312 + code + data
        // = 312 + 16 + 8 = 336
        let reloc_start = 312 + 16 + 8;
        // Two records of 8 bytes each = 16 bytes.
        let rec0 = &bytes[reloc_start..reloc_start + 8];
        let rec1 = &bytes[reloc_start + 8..reloc_start + 16];

        // rec0: r_address=0 (ADRP), r_info=0x3D000001
        //   sym=1, pcrel=1, length=2, extern=1, r_type=ARM64_RELOC_PAGE21 (3)
        let r0_addr = i32::from_le_bytes(rec0[0..4].try_into().unwrap());
        let r0_info = u32::from_le_bytes(rec0[4..8].try_into().unwrap());
        assert_eq!(r0_addr, 0, "ADRP byte offset");
        assert_eq!(r0_info, 0x3D000001, "PAGE21 packed info");

        // rec1: r_address=4 (ADD), r_info=0x4C000001
        //   sym=1, pcrel=0, length=2, extern=1, r_type=ARM64_RELOC_PAGEOFF12 (4)
        let r1_addr = i32::from_le_bytes(rec1[0..4].try_into().unwrap());
        let r1_info = u32::from_le_bytes(rec1[4..8].try_into().unwrap());
        assert_eq!(r1_addr, 4, "ADD byte offset");
        assert_eq!(r1_info, 0x4C000001, "PAGEOFF12 packed info");
    }

    #[test]
    fn globals_string_table_ends_with_twig_globals() {
        let bytes = arm64_globals_bytes(vec![0x00; 4], 1, &[]);
        // The string table always ends with "\0_twig_globals\0".
        let tail = b"_twig_globals\0";
        let end = &bytes[bytes.len() - tail.len()..];
        assert_eq!(end, tail, "string table must end with _twig_globals");
    }

    #[test]
    fn globals_rejects_non_arm64() {
        let result = pack_object_with_globals(&[0x00; 4], 0, 0, &[], &Target::macos_x64());
        assert!(result.is_err(), "x86_64 should be rejected");
    }

    #[test]
    fn globals_output_size_matches_formula() {
        // total = 312 + N + M + 2*R*8 + 2*16 + strtab_len
        // strtab = "\0_main\0_twig_globals\0" = 21 bytes
        let n = 12usize;   // code bytes
        let m = 2 * 8;     // 2 global slots
        let r = 2usize;    // 2 reloc entries → 4 records × 8 bytes
        let strtab_len = 1 + 5 + 1 + 13 + 1; // "\0_main\0_twig_globals\0" = 21
        let expected = 312 + n + m + 2 * r * 8 + 2 * 16 + strtab_len;
        let relocs = vec![
            GlobalByteReloc { adrp_byte_offset: 0, add_byte_offset: 4 },
            GlobalByteReloc { adrp_byte_offset: 8, add_byte_offset: 12 },
        ];
        let bytes = arm64_globals_bytes(vec![0x00u8; n], 2, &relocs);
        assert_eq!(bytes.len(), expected, "size formula");
    }

    // ── pack_object_with_globals_and_externals tests (LANG41) ─────────────────

    fn arm64_full(
        code: Vec<u8>,
        n_slots: usize,
        glob: &[GlobalByteReloc],
        ext: &[ExternBranchReloc],
    ) -> Vec<u8> {
        pack_object_with_globals_and_externals(
            &code, 0, n_slots, glob, ext, &Target::macos_arm64()
        ).unwrap()
    }

    #[test]
    fn full_no_externals_produces_macho_magic() {
        // Without any extern relocs, the function still produces a valid MH_OBJECT.
        let bytes = arm64_full(vec![0x00; 4], 0, &[], &[]);
        assert_eq!(&bytes[0..4], &[0xCF, 0xFA, 0xED, 0xFE], "Mach-O magic");
    }

    #[test]
    fn full_with_one_extern_emits_branch26_reloc() {
        // A single extern BL should produce exactly one BRANCH26 reloc record.
        // The reloc is emitted before the global relocs (extern BLs go first).
        //
        // Code: 8 bytes (2 instructions), extern BL at byte offset 0.
        let code = vec![0x00u8; 8];
        let ext = vec![ExternBranchReloc {
            byte_offset: 0,
            symbol: "__twig_print_i64".to_string(),
        }];
        let bytes = arm64_full(code, 0, &[], &ext);

        // Reloc block starts at 312 + 8 (text) + 0 (no data) = 320.
        // 1 reloc record = 8 bytes.
        let reloc_start = 312usize + 8;
        let r_addr = i32::from_le_bytes(bytes[reloc_start..reloc_start + 4].try_into().unwrap());
        let r_info = u32::from_le_bytes(bytes[reloc_start + 4..reloc_start + 8].try_into().unwrap());

        assert_eq!(r_addr, 0, "BL byte offset");

        // sym_idx=2, r_pcrel=1, r_length=2, r_extern=1, r_type=ARM64_RELOC_BRANCH26 (2)
        // = 2 | (1<<24) | (2<<25) | (1<<27) | (2<<28)
        // = 2 | 0x01000000 | 0x04000000 | 0x08000000 | 0x20000000
        // = 0x2D000002
        assert_eq!(r_info, 0x2D000002, "BRANCH26 packed info for sym_idx=2");
    }

    #[test]
    fn full_extern_symbol_emitted_as_n_undf() {
        // The external symbol must appear in the symbol table with n_type = 0x01
        // (N_UNDF | N_EXT) and n_sect = 0 (NO_SECT).
        let code = vec![0x00u8; 4];
        let ext = vec![ExternBranchReloc {
            byte_offset: 0,
            symbol: "__twig_print_i64".to_string(),
        }];
        let bytes = arm64_full(code, 0, &[], &ext);

        // Symbol table:
        //   index 0: _main       → nlist at symtab_off
        //   index 1: _twig_globals
        //   index 2: __twig_print_i64  ← we check this one
        //
        // symtab_off = 312 + code(4) + data(0) + relocs(1×8) = 324.
        let symtab_off = 312usize + 4 + 0 + 8;
        let ext_sym_off = symtab_off + 2 * 16; // skip _main and _twig_globals

        // nlist_64: n_strx(4) n_type(1) n_sect(1) n_desc(2) n_value(8)
        let n_type = bytes[ext_sym_off + 4];
        let n_sect = bytes[ext_sym_off + 5];
        let n_value = u64::from_le_bytes(
            bytes[ext_sym_off + 8..ext_sym_off + 16].try_into().unwrap()
        );

        assert_eq!(n_type, 0x01, "N_UNDF | N_EXT for undefined external");
        assert_eq!(n_sect, 0,    "n_sect must be 0 (NO_SECT) for undefined symbol");
        assert_eq!(n_value, 0,   "n_value must be 0 for undefined symbol");
    }

    #[test]
    fn full_output_size_formula_no_globals_one_extern() {
        // total = 312 (header) + N (text) + 0 (no data) + 1×8 (reloc) +
        //         3×16 (syms: _main + _twig_globals + extern) + strtab_len
        //
        // strtab = "\0_main\0_twig_globals\0__twig_print_i64\0"
        //   "\0"           = 1 byte  (leading NUL sentinel)
        //   "_main\0"      = 6 bytes
        //   "_twig_globals\0" = 14 bytes
        //   "__twig_print_i64\0" = 17 bytes  (16 chars + NUL)
        //                         ─────────
        //                   total = 38 bytes
        let n = 8usize;
        let strtab_len = 1 + 6 + 14 + 17; // 38
        let expected = 312 + n + 0 + 1 * 8 + 3 * 16 + strtab_len;
        let ext = vec![ExternBranchReloc {
            byte_offset: 0,
            symbol: "__twig_print_i64".to_string(),
        }];
        let bytes = arm64_full(vec![0x00u8; n], 0, &[], &ext);
        assert_eq!(bytes.len(), expected, "size formula for no-globals + 1 extern");
    }

    #[test]
    fn full_deduplicated_extern_symbols() {
        // Two BL instructions targeting the same external symbol must produce
        // only ONE N_UNDF symbol-table entry (deduplication).
        let code = vec![0x00u8; 16];
        let ext = vec![
            ExternBranchReloc { byte_offset: 0, symbol: "__twig_print_i64".to_string() },
            ExternBranchReloc { byte_offset: 4, symbol: "__twig_print_i64".to_string() },
        ];
        let bytes = arm64_full(code, 0, &[], &ext);

        // nsyms in LC_SYMTAB (4 bytes at offset 300 in the 2-section header) must be 3:
        //   _main + _twig_globals + 1 unique extern (not 4 = 2 + 2 dupes).
        //
        // 2-section layout: mach_header_64(32) + LC_SEGMENT_64+2sects(232)
        //   + LC_BUILD_VERSION(24) = 288 (LC_SYMTAB start)
        //   + cmd(4) + cmdsize(4) + symoff(4) = offset 300 for nsyms.
        let nsyms = u32::from_le_bytes(bytes[300..304].try_into().unwrap());
        assert_eq!(nsyms, 3, "deduplicated: only 1 unique extern symbol");
    }
}
