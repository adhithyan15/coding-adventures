# native-debug-info

DWARF 4 and CodeView 4 emission from a `DebugSidecar` for ELF, Mach-O, and PE binaries.

## What it does

After `code-packager` wraps native code in an ELF/Mach-O/PE binary, native debuggers
(gdb, lldb, WinDbg) cannot set breakpoints by source line or show variable names.
`native-debug-info` is a post-processing pass that reads a `DebugSidecar` (LANG13) and
embeds the appropriate platform-native debug sections:

| Binary format | Debug format | Debugger |
|---|---|---|
| ELF64 (Linux) | DWARF 4 | gdb, lldb, addr2line |
| Mach-O64 (macOS) | DWARF 4 in `__DWARF` segment | lldb, Xcode |
| PE32+ (Windows) | CodeView 4 in `.debug$S` | WinDbg, Visual Studio |

## Usage

```python
from debug_sidecar import DebugSidecarReader
from native_debug_info import DwarfEmitter, CodeViewEmitter, embed_debug_info

reader = DebugSidecarReader(sidecar_bytes)

# ELF (Linux)
emitter = DwarfEmitter(reader, load_address=0x400000,
                       symbol_table={"main": 0}, code_size=256)
elf_with_dwarf = emitter.embed_in_elf(elf_bytes)

# Mach-O (macOS)
macho_with_dwarf = emitter.embed_in_macho(macho_bytes)

# PE (Windows)
cv = CodeViewEmitter(reader, image_base=0x140000000,
                     symbol_table={"main": 0}, code_rva=0x1000)
pe_with_cv = cv.embed_in_pe(pe_bytes)

# One-call dispatch
enriched = embed_debug_info(packed_bytes, artifact, sidecar_bytes)
```

## Spec

[`code/specs/LANG14-native-debug-info.md`](../../../../specs/LANG14-native-debug-info.md)
