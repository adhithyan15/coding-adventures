# wasm-module-parser (Python)

Parses a raw `.wasm` binary file into a structured `WasmModule` object. This is
the decoder layer of the WASM toolchain — it takes bytes and produces data. It
does **not** execute, JIT-compile, or validate semantics beyond structural
correctness.

## Where this fits in the WASM stack

```
.wasm bytes
    ↓
wasm-module-parser    ← YOU ARE HERE (decode bytes → WasmModule)
    ↓
WasmModule (structured data from wasm-types)
    ↓
wasm-interpreter (execute) or wasm-linker (link)
```

## Dependencies

- `wasm-leb128` — LEB128 variable-length integer decoding (for all integer fields)
- `wasm-types` — Data structures: `WasmModule`, `FuncType`, `Import`, `Export`, etc.
- `wasm-opcodes` — Opcode definitions (available for downstream consumers)

## Installation

```bash
pip install -e ../wasm-leb128 -e ../wasm-types -e ../wasm-opcodes -e .
```

## Usage

```python
from wasm_module_parser import WasmModuleParser, WasmParseError

parser = WasmModuleParser()

# Parse a .wasm file
with open("program.wasm", "rb") as f:
    data = f.read()

try:
    module = parser.parse(data)
except WasmParseError as e:
    print(f"Parse failed at byte {e.offset}: {e.message}")
    raise

# Inspect the module
for ft in module.types:
    print(f"type: {ft.params} -> {ft.results}")

for imp in module.imports:
    print(f"import {imp.module_name}::{imp.name} ({imp.kind.name})")

for exp in module.exports:
    print(f"export '{exp.name}' [{exp.kind.name}] index={exp.index}")

if module.start is not None:
    print(f"start function: {module.start}")

for fb in module.code:
    print(f"function body: {len(fb.locals)} locals, {len(fb.code)} code bytes")
```

## Public API

### `WasmModuleParser`

```python
class WasmModuleParser:
    def parse(self, data: bytes) -> WasmModule: ...
```

- `data`: raw bytes of a `.wasm` file
- Returns: populated `WasmModule` (from `wasm_types`)
- Raises: `WasmParseError` on any malformed input

### `WasmParseError`

```python
class WasmParseError(Exception):
    message: str   # human-readable description
    offset: int    # byte offset in the input where the error was detected
```

### `WasmModule` fields (from `wasm_types`)

| Field       | Type                  | Section ID | Description                          |
|-------------|-----------------------|------------|--------------------------------------|
| `types`     | `list[FuncType]`      | 1          | Function signatures                  |
| `imports`   | `list[Import]`        | 2          | External entities imported           |
| `functions` | `list[int]`           | 3          | Type indices for local functions     |
| `tables`    | `list[TableType]`     | 4          | Table definitions                    |
| `memories`  | `list[MemoryType]`    | 5          | Memory definitions                   |
| `globals`   | `list[Global]`        | 6          | Global variable definitions          |
| `exports`   | `list[Export]`        | 7          | Exported entities                    |
| `start`     | `int \| None`         | 8          | Auto-called function index (or None) |
| `elements`  | `list[Element]`       | 9          | Table initialization segments        |
| `code`      | `list[FunctionBody]`  | 10         | Function bodies (bytecode)           |
| `data`      | `list[DataSegment]`   | 11         | Memory initialization segments       |
| `customs`   | `list[CustomSection]` | 0          | Custom (extension) sections          |

## WASM Binary Format Reference

```
┌─────────────────────────────────────────────────────────────────────┐
│  .wasm header (8 bytes)                                             │
│  [0x00 0x61 0x73 0x6D]  magic: "\0asm"                             │
│  [0x01 0x00 0x00 0x00]  version: 1 (little-endian u32)             │
├─────────────────────────────────────────────────────────────────────┤
│  Section envelope (repeating):                                      │
│  [section_id: u8]                                                   │
│  [section_size: u32 LEB128]                                         │
│  [payload: section_size bytes]                                      │
└─────────────────────────────────────────────────────────────────────┘
```

Sections 1–11 must appear in ascending ID order. Custom sections (ID 0) may
appear anywhere. All sections are optional.

## Running tests

```bash
cd code/packages/python/wasm-module-parser
uv venv --quiet --clear
uv pip install -e ../wasm-leb128 -e ../wasm-types -e ../wasm-opcodes -e ".[dev]" --quiet
uv run python -m pytest tests/ -v
```

Or use the BUILD/BUILD_windows scripts:

```bash
bash BUILD          # Linux/macOS
bash BUILD_windows  # Windows
```

## Test coverage

59 tests covering:
- Minimal module (header only)
- All 11 section types (Type, Import, Function, Table, Memory, Global, Export,
  Start, Element, Code, Data, Custom)
- Import kinds: function, table, memory, global
- Error paths: bad magic, wrong version, truncated header, truncated section
- Round-trip: manually-built binary → parse → verify all fields

Coverage: **96%**
