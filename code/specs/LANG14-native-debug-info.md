# LANG14 — native-debug-info: DWARF, CodeView, and dSYM for AOT Binaries

## Overview

When `aot-core` compiles a program and `code-packager` wraps it in an ELF,
Mach-O, or PE binary, the resulting file is opaque to native debuggers:
`gdb`, `lldb`, and WinDbg can execute it but cannot set breakpoints by source
line, cannot show variable names, and cannot print meaningful stack traces.

`native-debug-info` closes this gap.  It reads a `DebugSidecar` (LANG13) and
embeds the appropriate native debug format into the binary that `code-packager`
already produced:

| Binary format | Debug format | Debugger support |
|---|---|---|
| ELF64 (Linux) | DWARF 4 | `gdb`, `lldb`, `perf`, `addr2line`, `valgrind` |
| Mach-O64 (macOS) | DWARF 4 in `__DWARF` segment | `lldb`, Xcode, Instruments |
| PE32+ (Windows) | CodeView 4 in `.debug$S` section | WinDbg, Visual Studio, `dumpbin` |
| WASM | DWARF 4 in custom sections | Chrome DevTools, `wasm-objdump` |

The result: after `native-debug-info` runs, the binary is indistinguishable
from one compiled by clang or rustc — `gdb` can set breakpoints, `lldb` shows
local variable values, WinDbg prints function names.

---

## The full AOT debug pipeline

```
Source (.tetrad / .nib / …)
        │
        ▼
Compiler  ──────────────────────────────► IIRModule + DebugSidecar
        │
        ▼
aot-core.compile()  ───────────────────► CodeArtifact (native_bytes, symbol_table)
        │
        ▼
code-packager.pack()  ─────────────────► raw ELF / Mach-O / PE bytes  (no debug info)
        │
        ▼
native-debug-info.embed()  ────────────► ELF / Mach-O / PE with DWARF / CodeView
        │
   ┌────┴──────────────┐
   ▼                   ▼
 Linux binary       Windows binary
 + DWARF            + CodeView
   │
 gdb / lldb       WinDbg / VS
```

---

## DWARF for ELF (Linux) and Mach-O (macOS)

DWARF is the dominant open debug format.  Linux ELF and macOS Mach-O both use
it.  The difference is only in how it's packaged:

- **ELF**: DWARF lives in named sections (`.debug_line`, `.debug_info`, etc.)
  inside the ELF file itself.
- **Mach-O**: DWARF lives in a `__DWARF` segment containing sections named
  `__debug_line`, `__debug_info`, etc.  Apple's toolchain also produces a
  separate `.dSYM` bundle (a directory), but embedding directly in the binary
  is sufficient for `lldb` and Instruments.

### DWARF sections we emit (DWARF 4, minimal subset)

| Section | Content | Purpose |
|---|---|---|
| `.debug_abbrev` | Abbreviation table | Defines the structure of `.debug_info` entries |
| `.debug_info` | Compilation unit + `DW_TAG_subprogram` entries | Function names, file/line ranges |
| `.debug_line` | Line number program | Maps code address → (file, line, col) |
| `.debug_str` | String table | Deduplicated strings referenced by other sections |

This subset is sufficient for:
- Setting breakpoints by source line (`gdb break file.c:42`)
- Showing source in stack traces
- Printing function names in `addr2line` and `perf report`

**Not in scope for this implementation** (future PRs):
- `DW_TAG_variable` / `DW_TAG_formal_parameter` (variable values)
- `DW_TAG_base_type` (type descriptions)
- `DW_AT_location` expressions (register/memory locations of variables)

### `.debug_line` — Line Number Program

DWARF 4 line number programs use a stack machine with opcodes to compactly
encode the (address, file, line, col) mapping.  We use the simplest possible
encoding: `DW_LNS_advance_pc` + `DW_LNS_advance_line` + `DW_LNS_copy` for
every row.

```
Header:
  unit_length      u32     (length of this section minus 4)
  version          u16     (= 4 for DWARF 4)
  header_length    u32     (length of header after this field)
  minimum_instruction_length  u8  (= 1 for byte-addressed ISAs)
  maximum_ops_per_instruction u8  (= 1)
  default_is_stmt  u8      (= 1)
  line_base        i8      (= -5)
  line_range       u8      (= 14)
  opcode_base      u8      (= 13, standard opcodes 1–12 + our first special)
  standard_opcode_lengths  bytes[12]
  include_directories  (null-terminated list, ending with empty string)
  file_names  (null-terminated triplets: name, dir_index, mtime, size)
  (empty string terminates file name list)

Body (line number program opcodes):
  DW_LNS_set_file    (0x04) + ULEB128 file_index
  DW_LNS_advance_pc  (0x02) + ULEB128 address_delta
  DW_LNS_advance_line (0x03) + SLEB128 line_delta
  DW_LNS_copy        (0x01)    — emit one row
  DW_LNE_end_sequence (0x00 0x01 0x01)  — end marker
```

### `.debug_info` — Compilation Unit

```
Compilation unit header:
  unit_length    u32
  version        u16  (= 4)
  debug_abbrev_offset  u32  (= 0, our abbrev table starts at offset 0)
  address_size   u8   (= 8 for 64-bit targets)

Compilation unit DIE (DW_TAG_compile_unit, abbrev 1):
  DW_AT_producer   DW_FORM_strp  → "coding-adventures aot-core"
  DW_AT_language   DW_FORM_data2 → DW_LANG_C99 (0x0001, generic placeholder)
  DW_AT_name       DW_FORM_strp  → source file name
  DW_AT_comp_dir   DW_FORM_strp  → ""
  DW_AT_low_pc     DW_FORM_addr  → load_address
  DW_AT_high_pc    DW_FORM_data8 → code_size

For each function (DW_TAG_subprogram, abbrev 2):
  DW_AT_name       DW_FORM_strp  → function name
  DW_AT_decl_file  DW_FORM_data1 → file index (1-based)
  DW_AT_decl_line  DW_FORM_data4 → first source line
  DW_AT_low_pc     DW_FORM_addr  → function start address
  DW_AT_high_pc    DW_FORM_data8 → function byte length
  DW_AT_external   DW_FORM_flag_present

DW_TAG_null (0x00) — end of children
DW_TAG_null (0x00) — end of compile unit children
```

### `.debug_abbrev` — Abbreviation Table

```
Abbrev 1: DW_TAG_compile_unit, DW_CHILDREN_yes
  DW_AT_producer  DW_FORM_strp
  DW_AT_language  DW_FORM_data2
  DW_AT_name      DW_FORM_strp
  DW_AT_comp_dir  DW_FORM_strp
  DW_AT_low_pc    DW_FORM_addr
  DW_AT_high_pc   DW_FORM_data8
  0, 0  (terminator)

Abbrev 2: DW_TAG_subprogram, DW_CHILDREN_no
  DW_AT_name      DW_FORM_strp
  DW_AT_decl_file DW_FORM_data1
  DW_AT_decl_line DW_FORM_data4
  DW_AT_low_pc    DW_FORM_addr
  DW_AT_high_pc   DW_FORM_data8
  DW_AT_external  DW_FORM_flag_present
  0, 0  (terminator)

0  (end of abbreviation table)
```

---

## CodeView for PE (Windows)

Windows uses **CodeView 4** embedded in the PE binary as a `.debug$S` section.
WinDbg and Visual Studio read this without a separate `.pdb` file.

### `.debug$S` section structure

```
Signature:  u32 = 4  (CodeView 4)

Symbol subsection (DEBUG_S_SYMBOLS = 0xF1):
  subsection_type  u32  (= 0xF1)
  subsection_size  u32
  per symbol:
    record_length  u16
    record_type    u16
    payload        bytes[record_length - 2]

Line number subsection (DEBUG_S_LINES = 0xF2):
  subsection_type  u32  (= 0xF2)
  subsection_size  u32
  offset_of_contrib  u32   (RVA of code section)
  section_index      u16   (section number, 1-based)
  flags              u16   (= 0)
  code_size          u32
  per file block:
    file_checksum_offset  u32
    num_lines             u32
    block_size            u32
    per line:
      offset  u32   (byte offset from start of function)
      line_start  u32  (packed: bits 0–23 = line, bits 24–30 = col delta, bit 31 = is_statement)
```

### Symbol records we emit

**`S_GPROC32` (0x1110)** — global procedure:
```
  parent        u32  (= 0)
  end           u32  (offset to S_END)
  next          u32  (= 0)
  proc_len      u32  (byte length of function)
  dbg_start     u32  (= 0)
  dbg_end       u32  (= proc_len)
  type_index    u32  (= 0, no type info)
  offset        u32  (RVA of function start)
  segment       u16  (section index)
  flags         u8   (= 0)
  name          null-terminated string
```

**`S_END` (0x0006)** — closes `S_GPROC32`.

### `.debug$T` section

A minimal type section with just one entry: `LF_PROCEDURE` with no arguments
and return type `T_VOID`.  Without it some tools complain; with it they're
satisfied enough to show function names.

---

## Public API

```python
from native_debug_info import DwarfEmitter, CodeViewEmitter
from debug_sidecar import DebugSidecarReader

reader = DebugSidecarReader(sidecar_bytes)

# For Linux ELF or macOS Mach-O — same DWARF, different embedding
emitter = DwarfEmitter(
    reader=reader,
    load_address=0x400000,       # must match the packager's load_address
    symbol_table={"main": 0},    # function name → byte offset in code
    code_size=len(native_bytes),
)
dwarf_sections = emitter.build()
# => {".debug_abbrev": bytes, ".debug_info": bytes,
#     ".debug_line": bytes, ".debug_str": bytes}

# Embed into ELF
elf_with_debug = emitter.embed_in_elf(elf_bytes)

# Embed into Mach-O (same DWARF, __DWARF segment)
macho_with_debug = emitter.embed_in_macho(macho_bytes)

# For Windows PE — CodeView in .debug$S
cv_emitter = CodeViewEmitter(
    reader=reader,
    image_base=0x140000000,
    symbol_table={"main": 0},
    code_rva=0x1000,             # RVA of .text section
)
pe_with_debug = cv_emitter.embed_in_pe(pe_bytes)
```

### `CodeArtifact` integration

`native-debug-info` also provides a one-call convenience that reads the target
from a `CodeArtifact` and dispatches to the right emitter:

```python
from native_debug_info import embed_debug_info
from code_packager import PackagerRegistry

registry = PackagerRegistry.default()
packed_bytes = registry.pack(artifact)

# Enrich with debug info if a sidecar is available
if sidecar_bytes:
    packed_bytes = embed_debug_info(packed_bytes, artifact, sidecar_bytes)
```

---

## ULEB128 and SLEB128 encoding helpers

DWARF uses LEB128 (Little-Endian Base-128) variable-length integers throughout.
The package provides these as internal helpers:

```python
def encode_uleb128(value: int) -> bytes: ...
def encode_sleb128(value: int) -> bytes: ...
```

---

## Module layout

```
native-debug-info/
├── pyproject.toml
├── BUILD
├── README.md
├── CHANGELOG.md
└── src/
    └── native_debug_info/
        ├── __init__.py
        ├── dwarf.py        # DwarfEmitter — builds .debug_* sections
        ├── codeview.py     # CodeViewEmitter — builds .debug$S / .debug$T
        ├── embed.py        # embed_debug_info() — dispatches by target
        └── leb128.py       # ULEB128 / SLEB128 encoding
```

---

## Relationship to code-packager (LANG10)

`code-packager` stays format-agnostic — it knows nothing about DWARF or
CodeView.  `native-debug-info` is a post-processor that takes the packager's
output and enriches it.  This follows the same layered design:

```
code-packager   →  bare ELF / Mach-O / PE  (no debug info)
native-debug-info  →  same binary + DWARF / CodeView sections
```

The `code-packager` does not change.  The `CodeArtifact` does not need a
`sidecar` field.  Debug info is opt-in and applied as a separate pass.

---

## Testing strategy

- Unit tests for ULEB128 / SLEB128 encoding (all ranges, negative values).
- Unit tests for each DWARF section builder in isolation.
- Round-trip test: build DWARF sections from a known `DebugSidecarReader`,
  parse the resulting bytes with a minimal DWARF parser, verify the line table
  maps correctly.
- Unit tests for CodeView `.debug$S` builder — verify symbol record headers.
- Integration test: `DwarfEmitter.embed_in_elf()` on a known ELF; check magic,
  section count increase, section names present.
- Integration test: `CodeViewEmitter.embed_in_pe()` on a known PE; check
  `.debug$S` section presence.

Target: **95%+ line coverage**.
