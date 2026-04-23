# LANG13 — debug-sidecar: Source-Location Companion for the IIR Pipeline

## Overview

Every IIR compiler knows two things about each instruction it emits: what the
instruction does, and *where in the source file it came from*.  The first fact
travels in `IIRInstr.op`.  The second has nowhere to go today — so debuggers
can't map a paused VM back to a source line, insight tools can't say "line 3,
column 5" instead of "instruction index 2", and stack traces show register
numbers instead of variable names.

`debug-sidecar` fixes this.  It is the Python implementation of the binary
format specified in `05d-debug-sidecar-format.md`:

- A **writer** (`DebugSidecarWriter`) that compilers call once per emitted
  instruction to record `(instr_index, source_file, line, col)`.
- A **reader** (`DebugSidecarReader`) that debuggers and insight tools call to
  answer "given instruction index N in function F, what source location is that?"

The sidecar is intentionally **separate from IIRInstr** — keeping debug noise
off the hot execution path, exactly as DWARF is separate from ELF `.text` and
JavaScript source maps are separate from minified `.js`.

---

## What travels alongside the IIRModule

```
Compiler output:
  IIRModule          ← execution IR (IIRInstr, IIRFunction, …)
  DebugSidecar       ← source mapping (bytes, written by DebugSidecarWriter)

Consumer:
  vm-core            ← executes IIRModule
  debug-adapter      ← reads DebugSidecar to translate frame.ip → (file, line, col)
  vm-type-suggestions ← optionally reads DebugSidecar to enrich output with source locations
  jit-profiling-insights ← same
```

---

## Public API

### Writer

```python
from debug_sidecar import DebugSidecarWriter

writer = DebugSidecarWriter()

# Register the source file; returns a file_id
file_id = writer.add_source_file("fibonacci.tetrad", checksum=b"\x00" * 32)

# Called once per IIRInstr emitted by the compiler
writer.record(
    fn_name="fibonacci",
    instr_index=0,
    file_id=file_id,
    line=3,
    col=5,
)

# Register a function (execution unit)
writer.begin_function(
    fn_name="fibonacci",
    start_instr=0,
    param_count=1,
)
writer.end_function(fn_name="fibonacci", end_instr=12)

# Register a variable (parameter or local)
writer.declare_variable(
    fn_name="fibonacci",
    reg_index=0,
    name="n",
    type_hint="any",
    live_start=0,
    live_end=12,
)

# Serialise to bytes (store alongside IIRModule)
sidecar_bytes: bytes = writer.finish()
```

### Reader

```python
from debug_sidecar import DebugSidecarReader

reader = DebugSidecarReader(sidecar_bytes)

# Offset → source location (used by debugger when VM pauses)
loc = reader.lookup(fn_name="fibonacci", instr_index=2)
# => SourceLocation(file="fibonacci.tetrad", line=3, col=5)

# Source → instruction index (used to set breakpoints)
instr_index = reader.find_instr(file="fibonacci.tetrad", line=3)
# => 2

# Variable names at a given point (used for variable inspection panel)
vars = reader.live_variables(fn_name="fibonacci", at_instr=5)
# => [Variable(reg=0, name="n", type_hint="any")]

# All source files
files = reader.source_files()
# => ["fibonacci.tetrad"]
```

---

## Data model

### `SourceLocation`

```python
@dataclass(frozen=True)
class SourceLocation:
    file: str           # Source file path (from DebugSidecarWriter.add_source_file)
    line: int           # 1-based line number
    col: int            # 1-based column number
```

### `Variable`

```python
@dataclass(frozen=True)
class Variable:
    reg_index: int      # IIR register / slot index
    name: str           # Human name from source ("n", "result", …)
    type_hint: str      # Type annotation ("any", "u8", "Int", …)
    live_start: int     # First instr_index where this binding is valid
    live_end: int       # One-past-last instr_index
```

### `DebugSidecar` (internal, returned by `finish()` as bytes)

The binary layout follows spec `05d` exactly:
- Magic: `DBG\0` (4 bytes)
- Version: `u16 = 1`
- Section count + directory
- Section 0x01: String table (deduplicated null-terminated strings)
- Section 0x02: Source file table (path → SHA-256)
- Section 0x03: Line number table (per-function delta-encoded rows)
- Section 0x04: Execution unit table (function name, instr range, param count)
- Section 0x05: Variable table (reg_index, name, type_hint, live range)

For this implementation we use a **simplified in-memory format** — JSON
serialised to bytes — so the focus stays on the API and the pipeline
integration.  A follow-on PR can swap to the full binary format from spec `05d`
without changing any callers (the `finish()` / `DebugSidecarReader` boundary
encapsulates the format).

---

## Integration with compilers

Every IIR compiler adds approximately 3 lines per emitted instruction:

```python
class FibonacciCompiler:
    def compile(self, ast, source_path: str):
        writer = DebugSidecarWriter()
        file_id = writer.add_source_file(source_path)
        writer.begin_function("fibonacci", start_instr=0, param_count=1)
        writer.declare_variable("fibonacci", reg_index=0, name="n",
                                type_hint="any", live_start=0, live_end=12)

        instrs = []
        for node in ast.body:
            instr_index = len(instrs)
            writer.record("fibonacci", instr_index,
                          file_id=file_id, line=node.line, col=node.col)
            instrs.append(self._compile_node(node))

        writer.end_function("fibonacci", end_instr=len(instrs))
        module = IIRModule(name="fibonacci", functions=[...])
        sidecar = writer.finish()
        return module, sidecar
```

---

## Integration with insight tools

`vm-type-suggestions` and `jit-profiling-insights` accept an optional sidecar
to enrich their output with source locations:

```python
from debug_sidecar import DebugSidecarReader
from vm_type_suggestions import suggest

reader = DebugSidecarReader(sidecar_bytes)
report = suggest(fn_list, program_name="fibonacci", sidecar=reader)

# With sidecar:
#   ✅ fibonacci — 1,048,576 calls
#     'n' (arg 0) at fibonacci.tetrad:1:16: always u8
#     → declare 'n: u8'

# Without sidecar (current behaviour):
#   ✅ fibonacci — 1,048,576 calls
#     'n' (arg 0): always u8
#     → declare 'n: u8'
```

---

## Integration with LANG14 (native debug info)

`DebugSidecarReader` is the input to the LANG14 converters that produce
native debug info for AOT-compiled binaries:

```python
from debug_sidecar import DebugSidecarReader
from native_debug_info import DwarfEmitter, CodeViewEmitter

reader = DebugSidecarReader(sidecar_bytes)

# Produce DWARF sections for ELF (Linux) or Mach-O (macOS)
dwarf = DwarfEmitter(reader, load_address=0x400000)
elf_with_dwarf = dwarf.embed_in_elf(elf_bytes, symbol_table)

# Produce CodeView sections for PE (Windows)
cv = CodeViewEmitter(reader, image_base=0x140000000)
pe_with_cv = cv.embed_in_pe(pe_bytes, symbol_table)
```

---

## Module layout

```
debug-sidecar/
├── pyproject.toml
├── BUILD
├── README.md
├── CHANGELOG.md
└── src/
    └── debug_sidecar/
        ├── __init__.py
        ├── types.py       # SourceLocation, Variable
        ├── writer.py      # DebugSidecarWriter
        └── reader.py      # DebugSidecarReader
```

---

## Testing strategy

- Writer → finish() → Reader round-trip for all section types.
- `lookup(fn, instr_index)` returns correct source location.
- `find_instr(file, line)` returns correct instruction index.
- `live_variables(fn, at_instr)` returns only variables alive at that point.
- Multiple functions in one sidecar.
- Unknown instruction index returns `None` (not an exception).
- Multiple source files in one sidecar.

Target: **95%+ line coverage**.
