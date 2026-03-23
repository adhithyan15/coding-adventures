# wasm-types (Ruby)

Pure type definitions for the WebAssembly 1.0 type system. No parsing logic —
just data structures representing every type-level concept in WASM.

Used by `wasm_opcodes` and `wasm_module_parser` as the shared vocabulary for
everything that has a type in the WASM binary format.

## What it provides

### Constants / hashes

| Name               | Values                                       | Bytes                          |
|--------------------|----------------------------------------------|--------------------------------|
| `VALUE_TYPE`       | `:i32`, `:i64`, `:f32`, `:f64`              | `0x7F`, `0x7E`, `0x7D`, `0x7C`|
| `BLOCK_TYPE_EMPTY` | (constant)                                   | `0x40`                         |
| `EXTERNAL_KIND`    | `:function`, `:table`, `:memory`, `:global` | `0x00`–`0x03`                  |
| `FUNCREF`          | (constant)                                   | `0x70`                         |

### Structs

| Type            | Fields                                              | Purpose                        |
|-----------------|-----------------------------------------------------|--------------------------------|
| `FuncType`      | `params`, `results`                                 | Function signature             |
| `Limits`        | `min`, `max`                                        | Size bounds for memories/tables|
| `MemoryType`    | `limits`                                            | Linear memory type             |
| `TableType`     | `element_type`, `limits`                           | Table type                     |
| `GlobalType`    | `value_type`, `mutable`                            | Global variable type           |
| `Import`        | `module_name`, `name`, `kind`, `type_info`         | Import declaration             |
| `Export`        | `name`, `kind`, `index`                            | Export declaration             |
| `Global`        | `global_type`, `init_expr`                         | Module-defined global          |
| `Element`       | `table_index`, `offset_expr`, `function_indices`   | Table initializer segment      |
| `DataSegment`   | `memory_index`, `offset_expr`, `data`              | Memory initializer segment     |
| `FunctionBody`  | `locals`, `code`                                   | Function body                  |
| `CustomSection` | `name`, `data`                                     | Named arbitrary byte payload   |

### Mutable module container

| Type         | Purpose                                                    |
|--------------|------------------------------------------------------------|
| `WasmModule` | Holds all twelve section arrays plus `start` (Integer/nil) |

## How it fits in the stack

```
wasm_leb128        ← integer encoding/decoding
    ↓
wasm_types         ← THIS PACKAGE: all type definitions
    ↓
wasm_opcodes       ← instruction set definitions (uses VALUE_TYPE)
    ↓
wasm_module_parser ← parses .wasm binary into a WasmModule
    ↓
wasm_simulator     ← executes WasmModule
```

## Usage

```ruby
require "coding_adventures_wasm_types"

include CodingAdventures::WasmTypes

# Create a function type: (i32, i32) → i32
add_type = FuncType.new(
  [VALUE_TYPE[:i32], VALUE_TYPE[:i32]],
  [VALUE_TYPE[:i32]]
)

# Build a module skeleton
mod = WasmModule.new
mod.types     << add_type
mod.functions << 0               # function 0 uses type at index 0
mod.exports   << Export.new("add", :function, 0)
```

## Dependencies

- `coding_adventures_wasm_leb128` — LEB128 encoding

## Development

```bash
bundle install
bundle exec rake test
```
