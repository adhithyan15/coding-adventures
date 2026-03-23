# coding_adventures_wasm_opcodes

Complete WASM 1.0 opcode lookup table with metadata for all 172 instructions.

This gem is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
monorepo — a ground-up implementation of the computing stack from transistors to
operating systems.

## What it does

WebAssembly (WASM) is a binary instruction format. Every instruction is encoded
as a single opcode byte, followed by zero or more "immediate" operands. This gem
provides a complete lookup table with:

- The canonical text-format name (e.g. `i32.add`)
- The single-byte opcode value (e.g. `0x6A`)
- The instruction category (e.g. `numeric_i32`)
- The list of immediate operand types (e.g. `["memarg"]` for memory instructions)
- Stack effect metadata — how many values are consumed (`stack_pop`) and
  produced (`stack_push`)

## Where it fits in the stack

```
wasm_types        — WASM value types and type system primitives
    └── wasm_opcodes  — complete opcode table (this gem)
            └── wasm_leb128, wasm_module_parser, wasm_simulator, ...
```

## Installation

Add to your `Gemfile`:

```ruby
gem "coding_adventures_wasm_opcodes"
```

## Usage

```ruby
require "coding_adventures_wasm_opcodes"

M = CodingAdventures::WasmOpcodes

# Look up by byte value
info = M.get_opcode(0x6A)
# => #<struct OpcodeInfo name="i32.add", opcode=106, category="numeric_i32",
#       immediates=[], stack_pop=2, stack_push=1>

# Look up by name
info2 = M.get_opcode_by_name("i32.add")
# => same OpcodeInfo object

# Unknown byte → nil
M.get_opcode(0xFF)            # nil
M.get_opcode_by_name("foo")   # nil

# Iterate all opcodes
M::OPCODES.each do |byte, info|
  printf "0x%02X  %s\n", byte, info.name
end

# Check immediates
M.get_opcode_by_name("i32.load")&.immediates      # ["memarg"]
M.get_opcode_by_name("block")&.immediates          # ["blocktype"]
M.get_opcode_by_name("call_indirect")&.immediates  # ["typeidx", "tableidx"]
M.get_opcode_by_name("i32.add")&.immediates        # []
```

## API

### `OpcodeInfo` Struct

```ruby
OpcodeInfo = Struct.new(
  :name,       # String  — e.g. "i32.add"
  :opcode,     # Integer — e.g. 0x6A
  :category,   # String  — e.g. "numeric_i32"
  :immediates, # Array   — e.g. [], ["memarg"], ["blocktype"]
  :stack_pop,  # Integer — values consumed from stack
  :stack_push, # Integer — values produced onto stack
  keyword_init: true
)
```

### `OPCODES` — `Hash[Integer, OpcodeInfo]`

Primary lookup table keyed by opcode byte.

### `OPCODES_BY_NAME` — `Hash[String, OpcodeInfo]`

Secondary lookup table keyed by instruction name.

### `WasmOpcodes.get_opcode(byte)` → `OpcodeInfo | nil`

Look up an instruction by its opcode byte. Returns `nil` for reserved or
unknown bytes.

### `WasmOpcodes.get_opcode_by_name(name)` → `OpcodeInfo | nil`

Look up an instruction by its canonical text-format name. Returns `nil` if not found.

## Categories

| Category      | Description                                              | Count |
|---------------|----------------------------------------------------------|-------|
| `control`     | Structured control flow (block, loop, if, br, call, ...) | 13    |
| `parametric`  | Stack manipulation (drop, select)                        | 2     |
| `variable`    | Local/global variable access                             | 5     |
| `memory`      | Loads, stores, memory.size, memory.grow                  | 25    |
| `numeric_i32` | 32-bit integer arithmetic, comparisons, bitwise          | 30    |
| `numeric_i64` | 64-bit integer arithmetic, comparisons, bitwise          | 30    |
| `numeric_f32` | 32-bit float arithmetic and comparisons                  | 21    |
| `numeric_f64` | 64-bit float arithmetic and comparisons                  | 21    |
| `conversion`  | Type conversions between numeric types                   | 25    |
| **Total**     |                                                          | **172** |

## Immediate operand types

| Type            | Meaning                                                   |
|-----------------|-----------------------------------------------------------|
| `"i32"`         | 32-bit integer, LEB128-encoded                           |
| `"i64"`         | 64-bit integer, LEB128-encoded                           |
| `"f32"`         | 32-bit float, 4 bytes little-endian                      |
| `"f64"`         | 64-bit float, 8 bytes little-endian                      |
| `"blocktype"`   | Result type of a block (-0x40 for void, or a valtype)    |
| `"labelidx"`    | Branch target depth, LEB128-encoded                      |
| `"vec_labelidx"`| br_table: count + N label indices                        |
| `"funcidx"`     | Function table index, LEB128-encoded                     |
| `"typeidx"`     | Type section index                                        |
| `"tableidx"`    | Table section index (always 0 in WASM 1.0)               |
| `"localidx"`    | Local variable index                                      |
| `"globalidx"`   | Global variable index                                     |
| `"memarg"`      | `{ align: u32, offset: u32 }` — both LEB128-encoded      |
| `"memidx"`      | Memory index (always 0 in WASM 1.0)                      |

## Dependencies

- `coding_adventures_wasm_types` — WASM value types and type system primitives

## Development

```bash
bundle install
bundle exec rake test
```

## License

MIT
