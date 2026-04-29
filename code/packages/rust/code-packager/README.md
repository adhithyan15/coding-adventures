# code-packager

Cross-platform binary packaging for compiled code (LANG10).

`code-packager` is the **final stage of the ahead-of-time compilation pipeline**.
It takes a `CodeArtifact` — raw machine code bytes produced by any backend —
and wraps it in the appropriate OS-specific binary format. Cross-compilation
is first-class: a Mac can produce a Linux ELF, a CI machine can produce a
Windows PE, a Pi can produce a WASM module.

## Pipeline position

```text
aot-core.compile(module)
  → CodeArtifact(native_bytes, entry_point, target, symbol_table)
  │
  └──▶ PackagerRegistry::pack(&artifact)
         │
         ├── "elf64"     → Elf64Packager     → Linux ELF64 executable
         ├── "macho64"   → Macho64Packager   → macOS Mach-O 64 executable
         ├── "pe"        → PePackager        → Windows PE32+ executable
         ├── "wasm"      → WasmPackager      → WebAssembly module
         ├── "raw"       → RawPackager       → bare binary (embedded)
         └── "intel_hex" → IntelHexPackager  → Intel HEX ROM image
```

## Quick start

```rust
use code_packager::{CodeArtifact, PackagerRegistry, Target};

// A tiny x86-64 function: xor rax, rax; ret
let code = b"\x48\x31\xc0\xc3".to_vec();
let artifact = CodeArtifact::new(code, 0, Target::linux_x64());
let elf_bytes = PackagerRegistry::pack(&artifact).unwrap();
assert_eq!(&elf_bytes[..4], b"\x7fELF");
```

## Supported targets

| Factory method | `binary_format` | Packager |
|----------------|----------------|----------|
| `Target::linux_x64()` | `elf64` | `Elf64Packager` |
| `Target::linux_arm64()` | `elf64` | `Elf64Packager` |
| `Target::macos_x64()` | `macho64` | `Macho64Packager` |
| `Target::macos_arm64()` | `macho64` | `Macho64Packager` |
| `Target::windows_x64()` | `pe` | `PePackager` |
| `Target::wasm()` | `wasm` | `WasmPackager` |
| `Target::raw(arch)` | `raw` | `RawPackager` |
| `Target::intel_4004()` | `intel_hex` | `IntelHexPackager` |
| `Target::intel_8008()` | `intel_hex` | `IntelHexPackager` |

## Metadata keys

Packager-specific hints are passed through `CodeArtifact::metadata`:

| Key | Type | Packager | Description |
|-----|------|----------|-------------|
| `load_address` | `Int` | ELF64, Mach-O64 | Override virtual load address |
| `origin` | `Int` | Intel HEX | ROM start address (default 0) |
| `exports` | `List` | WASM | Function export names (first used) |

## Binary format details

### ELF64 (Linux)

Minimal single-segment ELF64 (`ET_EXEC`):
- 64-byte ELF header + 56-byte `PT_LOAD` program header + code
- Entry = `load_address + 120 + entry_point`
- Default `load_address = 0x400000`

### Mach-O 64-bit (macOS)

Minimal Mach-O executable (`MH_EXECUTE`):
- 32-byte `mach_header_64` + `LC_SEGMENT_64` + `section_64 __TEXT/__text` + `LC_MAIN`
- Default `load_address = 0x100000000` (arm64/x86_64 standard)

### PE32+ (Windows)

Minimal PE32+ console executable:
- 64-byte DOS stub + PE signature + 20-byte COFF header + 240-byte optional header
- One `.text` section at RVA `0x1000`, file-aligned to 512 bytes

### WASM

Minimal WASM 1.0 module wrapping the function body:
- Type: `() → i32`; single exported function named `"main"` (or first `exports` entry)

### Intel HEX

Standard Intel HEX records:
- Data records of up to 16 bytes each, with two's-complement checksum
- `origin` offset from metadata (default 0)

## Build

```bash
cargo build -p code-packager
cargo test -p code-packager
```

## Dependencies

| Crate | Use |
|-------|-----|
| `wasm-types` | `WasmModule`, `FuncType`, etc. |
| `wasm-module-encoder` | `encode_module` — WASM binary serialization |

## Tests

95 tests: 86 unit tests across all modules + 9 doc-tests.
