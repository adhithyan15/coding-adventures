# Changelog — native-debug-info

All notable changes to this crate will be documented here.

## [0.1.0] — 2026-04-28

### Added

- **`leb128` module** — ULEB128 and SLEB128 encode/decode used by the DWARF emitter.
  - `encode_uleb128(value: u64) -> Vec<u8>`
  - `encode_sleb128(value: i64) -> Vec<u8>`
  - `decode_uleb128(data: &[u8], offset: usize) -> (u64, usize)`
  - `decode_sleb128(data: &[u8], offset: usize) -> (i64, usize)`
  - 14 unit tests + 4 doc-tests.

- **`DwarfEmitter`** — DWARF 4 section builder.
  - `new(reader, load_address, symbol_table, code_size)` — constructs from a `DebugSidecarReader`.
  - `build()` — returns a `HashMap<String, Vec<u8>>` with keys `.debug_abbrev`,
    `.debug_info`, `.debug_line`, `.debug_str`.
  - `embed_in_elf(elf_bytes)` — parses ELF64 section header table, appends DWARF sections,
    updates `e_shoff` and section count, returns modified binary.
  - `embed_in_macho(macho_bytes)` — parses Mach-O 64-bit load commands, appends `__DWARF`
    segment with DWARF sections, returns modified binary.
  - 9 unit tests + 1 doc-test.

- **`CodeViewEmitter`** — CodeView 4 section builder.
  - `new(reader, image_base, symbol_table_u32, code_rva, code_section_index)`.
  - `build()` — returns `HashMap<String, Vec<u8>>` with keys `.debug$S`, `.debug$T`.
  - `embed_in_pe(pe_bytes)` — parses PE32+ section table, appends CodeView sections,
    updates COFF header section count, returns modified binary.
  - 7 unit tests + 1 doc-test.

- **`ArtifactInfo`** — descriptor struct for `embed_debug_info`.
  - Fields: `target: String`, `load_address: u64`, `image_base: u64`,
    `symbol_table_u64: HashMap<String, u64>`, `symbol_table_u32: HashMap<String, u32>`,
    `code_size: u64`, `code_rva: u32`, `code_section_index: u16`.

- **`embed_debug_info(packed_bytes, artifact, sidecar_bytes)`** — one-call convenience
  dispatcher that auto-detects the target platform from `artifact.target` (case-insensitive)
  and calls the correct emitter.
  - ELF targets: `linux`, `elf`, `freebsd`, `wasm`.
  - Mach-O targets: `macos`, `darwin`, `macho`.
  - PE targets: `windows`, `win32`, `pe`.
  - Returns `Err` for unknown targets or malformed sidecars.
  - 8 unit tests.

- Exported `encode_uleb128`, `decode_uleb128`, `encode_sleb128`, `decode_sleb128` from
  crate root for downstream packages that emit their own DWARF streams.

- Total: 42 tests (36 unit + 6 doc-tests).
