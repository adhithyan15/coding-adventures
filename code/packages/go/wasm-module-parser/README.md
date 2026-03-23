# wasm-module-parser (Go)

Parses a raw `.wasm` binary file into a structured `WasmModule` struct. This is
the decoder layer of the WASM toolchain — it takes bytes and produces data. It
does **not** execute, JIT-compile, or validate semantics beyond structural
correctness.

## Where this fits in the WASM stack

```
.wasm bytes
    ↓
wasm-module-parser    ← YOU ARE HERE (decode bytes → *WasmModule)
    ↓
*WasmModule (structured data from wasm-types)
    ↓
wasm-interpreter (execute) or wasm-linker (link)
```

## Dependencies

- `wasm-leb128` — LEB128 variable-length integer decoding
- `wasm-types` — Data structures: `WasmModule`, `FuncType`, `Import`, `Export`, etc.
- `wasm-opcodes` — Opcode definitions (available for downstream consumers)

## Installation

```bash
go get github.com/adhithyan15/coding-adventures/code/packages/go/wasm-module-parser
```

Or with local `replace` directives (monorepo usage):

```go
replace github.com/adhithyan15/coding-adventures/code/packages/go/wasm-module-parser => ../wasm-module-parser
```

## Usage

```go
import (
    wasmmoduleparser "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-module-parser"
    "errors"
    "os"
)

data, _ := os.ReadFile("program.wasm")

parser := wasmmoduleparser.New()
module, err := parser.Parse(data)
if err != nil {
    var pe *wasmmoduleparser.ParseError
    if errors.As(err, &pe) {
        fmt.Printf("parse failed at byte %d: %s\n", pe.Offset, pe.Message)
    }
    log.Fatal(err)
}

// Inspect the module
for _, ft := range module.Types {
    fmt.Printf("type: params=%v results=%v\n", ft.Params, ft.Results)
}
for _, imp := range module.Imports {
    fmt.Printf("import %s::%s\n", imp.ModuleName, imp.Name)
}
for _, exp := range module.Exports {
    fmt.Printf("export %q index=%d\n", exp.Name, exp.Index)
}
if module.Start != nil {
    fmt.Printf("start function: %d\n", *module.Start)
}
```

## Public API

### `New() *Parser`

Creates a new `Parser`. The parser is stateless; you can reuse it.

### `(*Parser).Parse(data []byte) (*WasmModule, error)`

Decodes a complete `.wasm` binary.

- `data`: raw bytes of a `.wasm` file
- Returns: populated `*WasmModule` on success, `nil` on error
- Error type: `*ParseError`

### `ParseError`

```go
type ParseError struct {
    Message string  // human-readable description
    Offset  int     // byte offset where the error was detected
}

func (e *ParseError) Error() string
```

### `WasmModule` fields (from `wasm-types`)

| Field       | Type               | Section ID | Description                          |
|-------------|--------------------|------------|--------------------------------------|
| `Types`     | `[]FuncType`       | 1          | Function signatures                  |
| `Imports`   | `[]Import`         | 2          | External entities imported           |
| `Functions` | `[]uint32`         | 3          | Type indices for local functions     |
| `Tables`    | `[]TableType`      | 4          | Table definitions                    |
| `Memories`  | `[]MemoryType`     | 5          | Memory definitions                   |
| `Globals`   | `[]Global`         | 6          | Global variable definitions          |
| `Exports`   | `[]Export`         | 7          | Exported entities                    |
| `Start`     | `*uint32`          | 8          | Auto-called function index (nil = absent) |
| `Elements`  | `[]Element`        | 9          | Table initialization segments        |
| `Code`      | `[]FunctionBody`   | 10         | Function bodies (bytecode)           |
| `Data`      | `[]DataSegment`    | 11         | Memory initialization segments       |
| `Customs`   | `[]CustomSection`  | 0          | Custom (extension) sections          |

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
cd code/packages/go/wasm-module-parser
go mod tidy
go test ./... -v -cover
```

Or use the BUILD script:

```bash
bash BUILD
```

## Test coverage

46 tests covering:
- Minimal module (header only)
- All 11 section types (Type, Import, Function, Table, Memory, Global, Export,
  Start, Element, Code, Data, Custom)
- Import kinds: function, table, memory, global
- Error paths: bad magic, wrong version, truncated header, truncated section
- Round-trip: manually-built binary → parse → verify all fields

Coverage: **81.6%**
