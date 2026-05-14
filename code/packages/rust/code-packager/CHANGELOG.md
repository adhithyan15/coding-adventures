# Changelog — code-packager

All notable changes to this crate will be documented here.

## [0.2.2] — 2026-05-13 (LANG41)

**External symbol support: `N_UNDF` entries + `ARM64_RELOC_BRANCH26` relocs
for unresolved `BL` targets; corrected ARM64 relocation type constants.**

### Added

- **`ExternBranchReloc`** struct — `(byte_offset: u32, symbol: String)` pair
  representing one unresolved `BL <extern>` instruction in the text section.
  Re-exported from crate root as `code_packager::ExternBranchReloc`.

- **`pack_object_with_globals_and_externals`** — supersedes
  `pack_object_with_globals`; always uses the 2-section layout (header = 312
  bytes) and additionally:
  - Deduplicates external symbols (first-appearance order); emits one
    `nlist_64` per unique symbol with `n_type = N_UNDF | N_EXT = 0x01`,
    `n_sect = 0`, `n_value = 0` — the standard Mach-O undefined-external
    sentinel that tells `ld` to resolve the symbol from archives or dylibs.
  - Emits one `ARM64_RELOC_BRANCH26` relocation record per `ExternBranchReloc`
    entry, packed as `sym_idx | 0x2D000000` (r_pcrel=1, r_length=2,
    r_extern=1, r_type=2).
  - Relocation order: BRANCH26 records first (externals), then PAGE21 +
    PAGEOFF12 pairs (globals).

### Fixed

- **ARM64 relocation type constants were incorrect** (LANG39 introduced the
  wrong values):
  - `ARM64_RELOC_PAGE21` corrected from `1` to `3`
  - `ARM64_RELOC_PAGEOFF12` corrected from `2` to `4`
  - New constant `ARM64_RELOC_BRANCH26 = 2` added
  - Constants now match Apple's `<mach-o/arm64/reloc.h>` enum exactly
  - Existing test expectations updated (`0x1D000001` → `0x3D000001` for
    PAGE21; `0x2C000001` → `0x4C000001` for PAGEOFF12)

### Tests added (5)

- `full_no_externals_produces_macho_magic` — round-trips through the new
  function with no globals and no externals; verifies magic bytes.
- `full_with_one_extern_emits_branch26_reloc` — confirms r_info =
  `0x2D000002` for the first external symbol (sym_idx = 2).
- `full_extern_symbol_emitted_as_n_undf` — verifies `n_type = 0x01`,
  `n_sect = 0`, `n_value = 0` in the symbol table entry.
- `full_output_size_formula_no_globals_one_extern` — asserts exact byte
  length via the formula `312 + relocs*8 + nsyms*16 + text + strtab`.
- `full_deduplicated_extern_symbols` — two `ExternBranchReloc` entries with
  the same symbol name produce exactly 3 nlist entries (not 4).

---

## [0.2.1] — 2026-05-13 (LANG39)

### Added

- **`GlobalByteReloc`** struct — byte-offset pair (`adrp_byte_offset`, `add_byte_offset`)
  for one `ADRP + ADD` instruction pair that references `_twig_globals`.
- **`pack_object_with_globals`** — extends `pack_object` to emit:
  - A `__DATA/__data` section (zero-initialised global variable slots)
  - An exported `_twig_globals` symbol pointing to the start of that section
  - `ARM64_RELOC_PAGE21` + `ARM64_RELOC_PAGEOFF12` relocation records per
    `GlobalByteReloc` so the system linker can patch `ADRP + ADD` pairs

  Header layout: `312` bytes (vs 232 in `pack_object`) due to the second `section_64`.
  ARM64-only (x86_64 global relocations are deferred).

9 new unit tests verify the Mach-O layout, relocation record encoding, and
output size formula.

## [0.2.0] — 2026-05-05

### Added

- **`macho_object` module** — produces Mach-O 64-bit relocatable
  **object files** (`MH_OBJECT`) suitable for feeding to Apple's
  system linker `ld`.  Used by `twig-aot` so the final executable is
  written by `ld` itself, granting it the trusted provenance modern
  macOS requires for `exec()`.

### Changed (`macho64` executable writer)

- Added `__PAGEZERO` segment (required by modern macOS).
- Switched the entry-point command from `LC_MAIN` to `LC_UNIXTHREAD`
  (no dyld required).
- Added `__LINKEDIT` segment + `LC_CODE_SIGNATURE` with an embedded
  ad-hoc SuperBlob / CodeDirectory signed via SHA-256 page hashes.
  Verified by `codesign --verify`.
- Added `LC_BUILD_VERSION` declaring macOS 15 minOS / SDK.
- Reworked tests to validate structural properties rather than fixed
  byte offsets (which now shift with the signature payload).

### Note

`code_packager::macho64::pack` produces a self-signed, structurally
valid Mach-O.  However, on macOS 15+ the kernel rejects executables
whose **last writer** is not a trusted system tool (provenance
sandbox), so the *direct* output is not always launchable.  Use
`macho_object::pack_object` + `ld` for that path.

## [0.1.0] — 2026-04-28

### Added

- **`Target`** — immutable description of a compilation target with factory methods:
  - `linux_x64()`, `linux_arm64()` → ELF64 targets
  - `macos_x64()`, `macos_arm64()` → Mach-O 64-bit targets
  - `windows_x64()` → PE32+ target
  - `wasm()` → WebAssembly target
  - `raw(arch)` → bare binary target (any arch)
  - `intel_4004()`, `intel_8008()` → Intel HEX ROM targets
  - `Display` impl: `"arch-os-binary_format"`

- **`CodeArtifact`** — handoff object between a compilation backend and a packager.
  - `native_bytes: Vec<u8>`, `entry_point: usize`, `target: Target`
  - `symbol_table: HashMap<String, usize>`, `metadata: HashMap<String, MetadataValue>`
  - Builder: `with_symbol_table()`, `with_metadata()`
  - Metadata accessors: `metadata_int()`, `metadata_str()`, `metadata_list()`

- **`MetadataValue`** — untyped metadata union: `Int(i64)`, `Str(String)`, `List(Vec<String>)`

- **`PackagerError`** — error enum:
  - `UnsupportedTarget(String)` — no packager handles this target
  - `WasmEncodeError(String)` — WASM module encoding failed

- **`elf64` module** — `Elf64Packager` producing a minimal ELF64 executable.
  - 64-byte ELF header + 56-byte `PT_LOAD` program header (one segment)
  - Entry virtual address = `load_address + 120 + entry_point`
  - `e_machine = 62` (x86_64) or `183` (AArch64) from `target.arch`
  - Default `load_address = 0x400000`; override via `metadata["load_address"]`
  - Supported: `linux_x64()`, `linux_arm64()`

- **`macho64` module** — `Macho64Packager` producing a minimal Mach-O 64-bit executable.
  - `mach_header_64` (32 bytes) + `LC_SEGMENT_64` + `section_64` `__TEXT/__text` + `LC_MAIN`
  - `cputype = 16777223` (x86_64) or `16777228` (ARM64)
  - Default `load_address = 0x100000000`; override via metadata
  - Supported: `macos_x64()`, `macos_arm64()`

- **`pe` module** — `PePackager` producing a minimal PE32+ executable.
  - 64-byte DOS stub + 4-byte PE signature + 20-byte COFF header + 240-byte optional header
  - One `.text` section at RVA `0x1000`, file alignment `0x200`
  - `ImageBase = 0x140000000`, section alignment `0x1000`
  - `AddressOfEntryPoint = 0x1000 + entry_point`
  - Supported: `windows_x64()`

- **`raw` module** — pass-through; returns `native_bytes` unchanged.
  - Any target with `binary_format == "raw"` is accepted.

- **`intel_hex` module** — Intel HEX encoder.
  - `encode_intel_hex(data: &[u8], origin: u16) -> String`
  - Data records of up to 16 bytes; two's-complement checksum per record
  - `origin` from `metadata["origin"]` (default 0)
  - Supported: `intel_4004()`, `intel_8008()`

- **`wasm` module** — wraps function body bytes in a minimal WASM module.
  - Type section: `() → i32`; single exported function
  - Export name from `metadata["exports"][0]` (default `"main"`)
  - Uses `wasm-module-encoder::encode_module`
  - Supported: `wasm()`

- **`PackagerRegistry`** — static dispatcher by `binary_format`.
  - `PackagerRegistry::pack(artifact)` — dispatches to the correct packager
  - `PackagerRegistry::file_extension(target)` — returns file suffix

- **95 tests**: 86 unit tests across all modules + 9 doc-tests.
