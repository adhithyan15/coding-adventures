# Changelog — coding-adventures-code-packager

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-23

### Added

**Target triple (`code_packager.target`)**
- `Target(arch, os, binary_format)` — frozen, hashable dataclass; usable
  as dict key and in frozensets.
- Factory methods: `linux_x64()`, `linux_arm64()`, `macos_x64()`,
  `macos_arm64()`, `windows_x64()`, `wasm()`, `raw(arch?)`,
  `intel_4004()`, `intel_8008()`.
- `__str__` renders as `"{arch}-{os}-{binary_format}"`.

**CodeArtifact (`code_packager.artifact`)**
- `CodeArtifact(native_bytes, entry_point, target, symbol_table?, metadata?)`
  — the handoff object between a compilation backend and the packager.
- `symbol_table` maps function name → byte offset within `native_bytes`.
- `metadata` is an open-ended dict for packager-specific hints (load address,
  subsystem flag, WASM imports, etc.).

**PackagerProtocol + PackagerRegistry (`code_packager.protocol`)**
- `PackagerProtocol` — structural (duck-typed) protocol; any object with
  `supported_targets`, `pack(artifact)`, and `file_extension(target)` qualifies.
- `PackagerRegistry` — maps `Target → PackagerProtocol`; `register()` stores a
  packager for every target it declares; `get(target)` raises
  `UnsupportedTargetError` if no packager matches; `pack(artifact)` is a
  one-call convenience.
- `PackagerRegistry.default()` — returns a registry pre-populated with all
  six built-in packagers.

**Exception hierarchy (`code_packager.errors`)**
- `PackagerError` — base class.
- `UnsupportedTargetError(target)` — carries `.target` attribute.
- `ArtifactTooLargeError(artifact_size, limit)` — carries both sizes.
- `MissingMetadataError(key)` — carries `.key` attribute.

**RawPackager (`code_packager.raw`)**
- Returns `native_bytes` verbatim; no container added.
- Accepts any `Target` with `binary_format="raw"`.
- Pre-registered for `raw()`, `raw(arch="i4004")`, `raw("i8008")`,
  `raw("x86_64")`, `raw("arm64")`, `raw("wasm32")`.
- Extension: `.bin`.

**IntelHexPackager (`code_packager.intel_hex`)**
- Wraps `native_bytes` in Intel HEX format (standard EPROM encoding).
- Delegates to `intel_4004_packager.encode_hex` (format-agnostic despite
  its name).
- Metadata key: `origin` (int, default 0) — ROM load address.
- Accepted targets: `intel_4004()`, `intel_8008()`.
- Extension: `.hex`.

**Elf64Packager (`code_packager.elf64`)**
- Produces a minimal valid ELF64 executable for Linux x86-64 and AArch64.
- Structure: 64-byte ELF header + 56-byte PT_LOAD program header + code.
- Sets `ET_EXEC`, `EM_X86_64` (62) or `EM_AARCH64` (183), `PT_LOAD`,
  `PF_R | PF_X`, `p_align = 0x200000`.
- Entry point virtual address = `load_address + header_size + entry_point`.
- Metadata key: `load_address` (int, default `0x400000`).
- Accepted targets: `linux_x64()`, `linux_arm64()`.
- Extension: `.elf`.

**MachO64Packager (`code_packager.macho64`)**
- Produces a minimal valid Mach-O 64-bit executable for macOS x86-64 and
  Apple Silicon (ARM64).
- Structure: 32-byte Mach-O header + `LC_SEGMENT_64` (152 bytes including
  one `__text` section entry) + `LC_UNIXTHREAD` + code.
- Uses `LC_UNIXTHREAD` (not `LC_MAIN`) to avoid dyld dependency.
- Entry point encoded in thread state: RIP field (x86-64) or PC field (ARM64).
- Metadata key: `load_address` (int, default `0x100000000`).
- Accepted targets: `macos_x64()`, `macos_arm64()`.
- Extension: `.macho`.

**PePackager (`code_packager.pe`)**
- Produces a minimal valid PE32+ executable (`.exe`) for Windows x86-64.
- Structure: 64-byte DOS stub + 4-byte `PE\0\0` signature + 20-byte COFF
  header + 240-byte optional header (core 112 bytes + 128 bytes of 16 empty
  data directories) + 40-byte `.text` section entry + code.
- `TimeDateStamp = 0` for reproducible builds.
- Section alignment: 4 KiB; file alignment: 512 bytes.
- Metadata keys: `subsystem` (int, default 3 = CUI), `image_base` (int,
  default `0x140000000`).
- Accepted targets: `windows_x64()`.
- Extension: `.exe`.

**WasmPackager (`code_packager.wasm`)**
- Wraps `native_bytes` (a function body expression) in a minimal WASM module
  with type section, function section, export section, and code section.
- Delegates to `wasm_module_encoder.encode_module`.
- Function type: `() → i32`.
- Metadata key: `exports` (list[str], default `["main"]`).
- Accepted targets: `wasm()`.
- Extension: `.wasm`.

**Package surface (`code_packager.__init__`)**
- Public exports: `Target`, `CodeArtifact`, `PackagerProtocol`,
  `PackagerRegistry`, `RawPackager`, `IntelHexPackager`, `Elf64Packager`,
  `MachO64Packager`, `PePackager`, `WasmPackager`, `PackagerError`,
  `UnsupportedTargetError`, `ArtifactTooLargeError`, `MissingMetadataError`.

### Tests

- 119 unit tests across 8 test modules; **100% line coverage**.
- `tests/conftest.py` — shared fixtures: `nop_x86`, `small_code`,
  `linux_artifact`, `windows_artifact`, `macos_arm64_artifact`,
  `macos_x64_artifact`, `raw_artifact`, `hex_artifact`, `wasm_artifact`.
- `tests/test_target.py` — 21 tests: factory methods, equality, hashability,
  frozenness, `__str__`.
- `tests/test_artifact.py` — 5 tests: default fields, symbol table,
  metadata, entry point offset.
- `tests/test_errors.py` — 12 tests: exception hierarchy, message content,
  attributes, raisability.
- `tests/test_protocol.py` — 6 tests: protocol conformance, registry
  register/get/pack, unsupported target, default registry completeness,
  overwrite behavior.
- `tests/test_raw.py` — 6 tests: pass-through, empty/large blobs, extension,
  wrong-format error, all arch variants.
- `tests/test_intel_hex.py` — 9 tests: round-trip, ASCII output, EOF record,
  8008 target, origin metadata, extension, wrong-format error, single byte.
- `tests/test_elf64.py` — 17 tests: magic, class, endianness, type, machine
  (x86-64 and ARM64), entry point, phoff, phnum, flags, embedded code, load
  address override, extension, wrong-target error, total size, filesz, align.
- `tests/test_macho64.py` — 14 tests: magic, CPU types, filetype, flags,
  ncmds, load command presence (LC_SEGMENT_64 + LC_UNIXTHREAD), code
  embedding, load address override, extension, wrong-target error, thread
  state entry point.
- `tests/test_pe.py` — 18 tests: MZ magic, PE signature, machine type,
  section count, timestamp, PE32+ magic, subsystem (default and GUI override),
  entry point RVA, offset application, embedded code, image base override,
  header alignment, image size alignment, extension, wrong-target error,
  section name.
- `tests/test_wasm.py` — 10 tests: magic, version, byte output, non-empty,
  code present, custom export name, default export, extension, wrong-target
  error, empty exports fallback.
