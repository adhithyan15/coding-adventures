# Changelog — code-packager

All notable changes to this crate will be documented here.

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
