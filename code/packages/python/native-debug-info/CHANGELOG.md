# Changelog

All notable changes to `coding-adventures-native-debug-info` are documented here.

## [0.1.0] — 2026-04-23

### Added

- `leb128` module — ULEB128 and SLEB128 encode/decode for DWARF variable-length integers
- `DwarfEmitter` — builds DWARF 4 debug sections from a `DebugSidecarReader`:
  - `.debug_str` — null-terminated string table with deduplication
  - `.debug_abbrev` — compile-unit and subprogram abbreviation table
  - `.debug_line` — DWARF 4 line number program (stack machine) with `DW_LNE_set_address`,
    `DW_LNS_advance_pc`, `DW_LNS_advance_line`, `DW_LNS_copy`, `DW_LNE_end_sequence`
  - `.debug_info` — compile unit DIE + one subprogram DIE per function
  - `embed_in_elf(elf_bytes)` — appends 4 new sections to an ELF64 binary, updates `.shstrtab`,
    `e_shoff`, and `e_shnum`
  - `embed_in_macho(macho_bytes)` — inserts an `LC_SEGMENT_64` load command for the `__DWARF`
    segment (4 sections), shifts all existing section file offsets forward by 392 bytes
- `CodeViewEmitter` — builds CodeView 4 debug sections for PE32+ (Windows):
  - `.debug$S` — `DEBUG_S_STRINGTABLE` (file path strings) + `DEBUG_S_FILECHKSMS` (checksums) +
    `DEBUG_S_SYMBOLS` (`S_GPROC32` per function) + `DEBUG_S_LINES` (line mappings)
  - `.debug$T` — minimal type section with one `LF_PROCEDURE` record
  - `embed_in_pe(pe_bytes)` — appends `.debug$S` and `.debug$T` sections to a PE32+ binary,
    validates header space for 2 new section headers, updates `NumberOfSections` and `SizeOfImage`
- `embed_debug_info(packed_bytes, artifact, sidecar_bytes)` — one-call dispatch:
  - `linux`, `elf`, `freebsd`, `wasm` → `DwarfEmitter.embed_in_elf`
  - `macos`, `darwin`, `macho` → `DwarfEmitter.embed_in_macho`
  - `windows`, `win32`, `pe` → `CodeViewEmitter.embed_in_pe`
  - Target string is case-insensitive; unknown targets raise `ValueError`
- 111 unit tests across `test_leb128`, `test_dwarf`, `test_codeview`, `test_embed`
- `conftest.py` fixtures: `fibonacci_reader`, `minimal_elf64`, `minimal_macho64`, `minimal_pe32plus`

### Implementation notes

- The LANG14 spec's compile-unit abbreviation was extended with `DW_AT_stmt_list`
  (`DW_FORM_sec_offset`) so that debuggers can locate the `.debug_line` section. This diverges
  from the spec, which is updated accordingly.
- `DwarfEmitter` accesses `reader._raw_line_table` directly (companion-package pattern) to avoid
  materialising `SourceLocation` objects for every instruction during binary encoding.
- Mach-O section insertion shifts all `LC_SEGMENT_64` file offsets by the load command size (392
  bytes = 72-byte `LC_SEGMENT_64` header + 4 × 80-byte `section_64`).
- CodeView `DEBUG_S_FILECHKSMS` references offsets within the `DEBUG_S_STRINGTABLE` subsection
  that immediately precedes it; both subsections must be present for debuggers to resolve file names.
