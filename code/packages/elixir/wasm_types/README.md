# wasm_types (Elixir)

Pure type definitions for the WebAssembly 1.0 (MVP) type system.

This package contains **no parsing logic**. It defines Elixir structs and
helper functions that represent a decoded WASM module's type information.
Higher-level packages (`wasm_opcodes`, `wasm_module_parser`) depend on these
definitions.

## What it provides

### Helper functions (`CodingAdventures.WasmTypes`)

| Function              | Returns                                          |
|-----------------------|--------------------------------------------------|
| `value_type(:i32)`    | `0x7F` — 32-bit integer byte tag                 |
| `value_type(:i64)`    | `0x7E` — 64-bit integer byte tag                 |
| `value_type(:f32)`    | `0x7D` — 32-bit float byte tag                   |
| `value_type(:f64)`    | `0x7C` — 64-bit float byte tag                   |
| `block_type_empty()`  | `0x40` — empty block type byte tag               |
| `external_kind(:function)` | `0x00`                                    |
| `external_kind(:table)`    | `0x01`                                    |
| `external_kind(:memory)`   | `0x02`                                    |
| `external_kind(:global)`   | `0x03`                                    |

### Struct modules

| Module                                      | Description                              |
|---------------------------------------------|------------------------------------------|
| `CodingAdventures.WasmTypes.FuncType`       | Function signature (params + results)    |
| `CodingAdventures.WasmTypes.Limits`         | min/max size for memories and tables     |
| `CodingAdventures.WasmTypes.MemoryType`     | Linear memory declaration                |
| `CodingAdventures.WasmTypes.TableType`      | Function reference table declaration     |
| `CodingAdventures.WasmTypes.GlobalType`     | Global variable type + mutability        |
| `CodingAdventures.WasmTypes.Import`         | Import from the host environment         |
| `CodingAdventures.WasmTypes.Export`         | Export to the host environment           |
| `CodingAdventures.WasmTypes.Global`         | Module-defined global with init expr     |
| `CodingAdventures.WasmTypes.Element`        | Table initialization segment             |
| `CodingAdventures.WasmTypes.DataSegment`    | Memory initialization segment            |
| `CodingAdventures.WasmTypes.FunctionBody`   | Function locals + bytecode               |
| `CodingAdventures.WasmTypes.CustomSection`  | Named tool metadata section              |
| `CodingAdventures.WasmTypes.WasmModule`     | Top-level container for all sections     |

## How it fits in the stack

```
coding_adventures_wasm_leb128     ← LEB128 integer decoding
coding_adventures_wasm_types      ← THIS PACKAGE: type definitions (no parsing)
coding_adventures_wasm_opcodes    ← instruction set definitions
coding_adventures_wasm_module_parser  ← binary → WasmModule
coding_adventures_wasm_simulator  ← execution engine
```

## Usage

```elixir
alias CodingAdventures.WasmTypes
alias CodingAdventures.WasmTypes.{FuncType, Limits, MemoryType, WasmModule}

# Get the binary byte tag for i32
WasmTypes.value_type(:i32)  # => 0x7F

# Describe a function: (i32, i64) -> f32
sig = %FuncType{params: [:i32, :i64], results: [:f32]}

# Describe a memory: at least 1 page, at most 4 pages
mem = %MemoryType{limits: %Limits{min: 1, max: 4}}

# Start with an empty module
module = %WasmModule{}

# "Update" it immutably (Elixir pattern)
module = %{module | types: [sig], memories: [mem]}
```

## Import type_info convention

The `Import.type_info` field uses tagged tuples:

```elixir
{:function, type_index}          # e.g., {:function, 0}
{:table, %TableType{...}}
{:memory, %MemoryType{...}}
{:global, %GlobalType{...}}
```

## WASM binary encoding quick reference

```
ValueType bytes:   i32=0x7F  i64=0x7E  f32=0x7D  f64=0x7C
ExternalKind:      func=0x00  table=0x01  mem=0x02  global=0x03
BlockType empty:   0x40
FuncType tag:      0x60
FuncRef tag:       0x70  (TableType default element_type)
Limits flags:      0x00 (min only)  0x01 (min + max)
```

## Dependencies

- `coding_adventures_wasm_leb128` — LEB128 encoding/decoding

## Development

```bash
cd code/packages/elixir/wasm_types
mix deps.get
mix test

# Or use the BUILD script
bash BUILD
```
