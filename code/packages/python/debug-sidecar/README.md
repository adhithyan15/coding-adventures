# debug-sidecar

Source-location companion for the IIR pipeline. Maps instruction indices to
file/line/col for debuggers, insight tools, and native debug info emitters
(DWARF, CodeView).

## What it does

The IIR (`interpreter-ir`) pipeline keeps `IIRInstr` lean — no source
positions stored on hot-path objects. `debug-sidecar` is the *separate
channel* that carries that information forward, analogous to:

- DWARF sections alongside ELF `.text`
- Source maps alongside minified JavaScript
- PDB files alongside Windows PE executables

A compiler calls `DebugSidecarWriter` once per emitted instruction to record
the mapping. After compilation, `finish()` returns an opaque `bytes` object.
Any downstream consumer — a debugger adapter, an insight tool, a DWARF
emitter — loads those bytes into `DebugSidecarReader` and queries them.

## Stack position

```
Source (.tetrad)
    │
    ▼
Compiler  ──── DebugSidecarWriter ─── finish() ──► bytes
    │                                                │
    ▼                                                ▼
IIRFunction / IIRInstr              DebugSidecarReader
                                         │
                            ┌────────────┼────────────┐
                            ▼            ▼             ▼
                        debugger    jit-profiling   native-debug-info
                        adapter       insights       (DWARF/CodeView)
```

## Usage

### Writing (compiler side)

```python
from debug_sidecar import DebugSidecarWriter

writer = DebugSidecarWriter()
file_id = writer.add_source_file("fibonacci.tetrad")

writer.begin_function("fibonacci", start_instr=0, param_count=1)
writer.declare_variable("fibonacci", reg_index=0, name="n",
                        type_hint="any", live_start=0, live_end=12)

for instr_index, (instr, node) in enumerate(zip(instructions, ast_nodes)):
    writer.record("fibonacci", instr_index,
                  file_id=file_id, line=node.line, col=node.col)

writer.end_function("fibonacci", end_instr=12)
sidecar: bytes = writer.finish()
```

### Reading (debugger / tool side)

```python
from debug_sidecar import DebugSidecarReader

reader = DebugSidecarReader(sidecar_bytes)

# Offset → source  (debugger paused at instruction 7)
loc = reader.lookup("fibonacci", 7)
if loc:
    print(f"Stopped at {loc}")   # "fibonacci.tetrad:3:5"

# Source → offset  (setting a breakpoint on line 10)
idx = reader.find_instr("fibonacci.tetrad", 10)
if idx is not None:
    vm.set_breakpoint(idx, "fibonacci")

# Variable inspection
for var in reader.live_variables("fibonacci", 5):
    print(f"  {var.name} (reg {var.reg_index}): {var.type_hint}")
```

## API reference

### `DebugSidecarWriter`

| Method | Description |
|---|---|
| `add_source_file(path, checksum=b"") → int` | Register a source file; returns file_id (idempotent) |
| `begin_function(fn_name, *, start_instr, param_count)` | Mark start of a function's instruction range |
| `end_function(fn_name, *, end_instr)` | Mark end of a function's instruction range |
| `record(fn_name, instr_index, *, file_id, line, col)` | Map one instruction to its source location |
| `declare_variable(fn_name, *, reg_index, name, type_hint, live_start, live_end)` | Register a named variable binding |
| `finish() → bytes` | Serialize to opaque bytes |

### `DebugSidecarReader`

| Method | Description |
|---|---|
| `lookup(fn_name, instr_index) → SourceLocation \| None` | Offset → source (DWARF-style, nearest preceding record) |
| `find_instr(file, line) → int \| None` | Source → first matching instruction index |
| `live_variables(fn_name, at_instr) → list[Variable]` | Variables live at an instruction, sorted by reg_index |
| `source_files() → list[str]` | All registered source file paths |
| `function_names() → list[str]` | All functions with debug info |
| `function_range(fn_name) → tuple[int, int] \| None` | (start_instr, end_instr) for a function |

### `SourceLocation`

Frozen dataclass: `file: str`, `line: int`, `col: int`. `str()` returns
`"file:line:col"`.

### `Variable`

Frozen dataclass: `reg_index`, `name`, `type_hint`, `live_start`, `live_end`.
`is_live_at(instr_index) → bool`.

## Internal format

The sidecar is JSON serialized to UTF-8 bytes. This is intentionally simple
for the initial implementation — it lets the full pipeline work while the
compact binary format described in spec `05d` is finalized. The `finish()` /
`DebugSidecarReader` boundary is the only place that knows the format, so
swapping to binary later is a single-file change.

## Spec

[`code/specs/LANG13-debug-sidecar.md`](../../../../specs/LANG13-debug-sidecar.md)
