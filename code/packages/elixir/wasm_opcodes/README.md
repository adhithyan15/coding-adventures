# wasm_opcodes (Elixir)

A complete compile-time lookup table for all 172 WebAssembly 1.0 MVP instructions,
with metadata for each opcode: name, byte value, category, immediates, and stack effects.

Part of the [coding-adventures](../../../../README.md) monorepo — a ground-up
implementation of the computing stack from transistors to operating systems.

## What it does

Every WASM instruction is identified by a single opcode byte. This module provides:

- `@opcodes` module attribute — all 172 WASM 1.0 instructions as a list of maps,
  evaluated once at compile time (zero runtime initialization cost)
- `get_opcode/1` — look up an instruction by byte value
- `get_opcode_by_name/1` — look up by canonical text name (e.g. `"i32.add"`)
- `all_opcodes/0` — return the full list

## Data structure

Each opcode is a plain Elixir map:

```elixir
%{
  name:       "i32.add",    # canonical text name
  opcode:     0x6A,         # byte value
  category:   "numeric_i32",# instruction group
  immediates: [],           # list of immediate argument names
  stack_pop:  2,            # values consumed from operand stack
  stack_push: 1             # values produced onto operand stack
}
```

## Usage

```elixir
alias CodingAdventures.WasmOpcodes

# Look up by byte value
{:ok, op} = WasmOpcodes.get_opcode(0x6A)
op.name       # => "i32.add"
op.stack_pop  # => 2
op.stack_push # => 1

# Look up by name
{:ok, op} = WasmOpcodes.get_opcode_by_name("i32.const")
op.opcode      # => 0x41
op.immediates  # => ["i32"]

# Unknown byte or name
WasmOpcodes.get_opcode(0xFF)              # => {:error, :unknown_opcode}
WasmOpcodes.get_opcode_by_name("banana")  # => {:error, :unknown_opcode}

# All opcodes
ops = WasmOpcodes.all_opcodes()
length(ops)  # => 172

# Filter by category
memory_ops = Enum.filter(ops, &(&1.category == "memory"))
```

## Categories

| Category      | Description                                 | Example instructions          |
|---------------|---------------------------------------------|-------------------------------|
| `control`     | Program flow, calls, branches               | `unreachable`, `call`, `br`   |
| `parametric`  | Type-agnostic stack operations              | `drop`, `select`              |
| `variable`    | Local and global variable access            | `local.get`, `global.set`     |
| `memory`      | Loads, stores, memory size/grow             | `i32.load`, `i64.store8`      |
| `numeric_i32` | 32-bit integer arithmetic and comparisons   | `i32.add`, `i32.lt_s`         |
| `numeric_i64` | 64-bit integer arithmetic and comparisons   | `i64.mul`, `i64.ge_u`         |
| `numeric_f32` | 32-bit float arithmetic and comparisons     | `f32.sqrt`, `f32.copysign`    |
| `numeric_f64` | 64-bit float arithmetic and comparisons     | `f64.add`, `f64.nearest`      |
| `conversion`  | Type conversions between numeric types      | `i32.wrap_i64`, `f64.promote_f32` |

## How it fits in the stack

```
wasm_types         ← type system (FuncType, ValueType, etc.)
wasm_leb128        ← integer encoding used in WASM binaries
wasm_opcodes       ← this module: instruction metadata
wasm_module_parser ← parses .wasm binary files (uses all three above)
```

## Dependencies

- `coding_adventures_wasm_types` — for shared type definitions

## Development

```bash
# Run tests
cd code/packages/elixir/wasm_opcodes
mix deps.get --quiet && mix test

# Or via the BUILD file
bash BUILD
```

## Opcode count note

WASM 1.0 MVP defines exactly **172** instructions. The byte range 0x00–0xBF
contains gaps (e.g. 0x06–0x0A, 0x12–0x1F, 0x25–0x27) that are
reserved/unassigned. The "~183" figure sometimes cited includes post-MVP
proposals (SIMD, bulk-memory, threads) that use a 0xFC prefix encoding.
