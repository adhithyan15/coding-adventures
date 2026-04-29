# native-debug-info

DWARF 4 and CodeView 4 debug-section emitter (LANG14).

`native-debug-info` converts source-location data from a [`DebugSidecarReader`] into
the native debug sections understood by gdb/lldb (DWARF 4) on Linux/macOS and
WinDbg/Visual Studio (CodeView 4) on Windows.

## Pipeline position

```text
aot-core.compile(module) → .aot binary + sidecar bytes
  │
  ↓ DebugSidecarReader::new(sidecar_bytes)
  │
  ├── DwarfEmitter::embed_in_elf(elf_bytes)     → ELF + DWARF
  ├── DwarfEmitter::embed_in_macho(macho_bytes) → Mach-O + DWARF
  └── CodeViewEmitter::embed_in_pe(pe_bytes)    → PE + CodeView
```

## Public API

| Item | Role |
|------|------|
| `DwarfEmitter` | Builds `.debug_abbrev`, `.debug_info`, `.debug_line`, `.debug_str`; embeds them into ELF64 or Mach-O 64-bit binaries. |
| `CodeViewEmitter` | Builds `.debug$S` and `.debug$T`; embeds them into a PE32+ binary. |
| `embed_debug_info` | Convenience dispatcher — auto-detects target from `ArtifactInfo.target` and calls the correct emitter. |
| `ArtifactInfo` | Descriptor providing target platform, load address, image base, and symbol tables for both DWARF (u64) and CodeView (u32). |
| `encode_uleb128` / `decode_uleb128` | ULEB128 encode/decode used by the DWARF emitter; exported for downstream packages. |
| `encode_sleb128` / `decode_sleb128` | SLEB128 encode/decode; used by DWARF for signed values. |

## Quick start

```rust
use native_debug_info::{encode_uleb128, decode_uleb128};

let encoded = encode_uleb128(624485);
let (decoded, _) = decode_uleb128(&encoded, 0);
assert_eq!(decoded, 624485);
```

## Target platform dispatch

`embed_debug_info` recognises these target strings (case-insensitive):

| Target strings | Emitter |
|---------------|---------|
| `linux`, `elf`, `freebsd`, `wasm` | `DwarfEmitter::embed_in_elf` |
| `macos`, `darwin`, `macho` | `DwarfEmitter::embed_in_macho` |
| `windows`, `win32`, `pe` | `CodeViewEmitter::embed_in_pe` |

Unknown targets return `Err("unsupported target platform: ...")`.

## DWARF 4 section layout

| Section | Contents |
|---------|----------|
| `.debug_abbrev` | Abbreviation table: one `DW_TAG_compile_unit` + one `DW_TAG_subprogram` entry. |
| `.debug_str` | Null-terminated strings: compilation directory + source file names. |
| `.debug_info` | Compile-unit header → `DW_TAG_compile_unit` DIE → one `DW_TAG_subprogram` DIE per function. |
| `.debug_line` | DWARF 4 line-number program header + `DW_LNS_*` opcodes derived from `raw_line_rows()`. |

## CodeView 4 section layout

| Section | Contents |
|---------|----------|
| `.debug$S` | Symbol subsection: `S_GPROC32` records for each function from the symbol table. |
| `.debug$T` | Type subsection: a single `LF_ARGLIST` + `LF_PROCEDURE` leaf covering the whole module. |

## LEB128 encoding

DWARF uses **LEB128** (Little-Endian Base-128) for compact integer encoding:

- **ULEB128** — unsigned: 7 bits per byte, MSB = continuation flag.
- **SLEB128** — signed: same layout but sign-extended from the final group of 7 bits.

```rust
// 624485 encodes as [0xe5, 0x8e, 0x26]
let bytes = encode_uleb128(624485);
assert_eq!(bytes, vec![0xe5, 0x8e, 0x26]);
```

## Build

```bash
cargo build -p native-debug-info
cargo test -p native-debug-info
```

## Dependencies

| Crate | Use |
|-------|-----|
| `debug-sidecar` | `DebugSidecarReader` + `LineRow` — source for all debug data |

## Tests

42 tests: 36 unit tests across `leb128`, `dwarf`, `codeview`, `embed`, plus 6 doc-tests.
