# coding-adventures-wasm-simulator

WebAssembly interpreter / simulator for Lua.

Executes WebAssembly modules by interpreting bytecode on a software-emulated
stack machine. Accepts parsed modules from
[coding-adventures-wasm-module-parser](../wasm_module_parser/) and runs them.

## Part of the coding-adventures stack

This library sits one layer above the parser:

```
wasm_leb128          — LEB128 variable-length integer encoding
wasm_types           — Value type constants (i32, i64, f32, f64, ...)
wasm_opcodes         — Opcode name/metadata table
wasm_module_parser   — Binary .wasm parser → structured Lua table
wasm_simulator       — Bytecode executor (this package)
```

## Installation

```bash
luarocks make --local coding-adventures-wasm-simulator-0.1.0-1.rockspec
```

## Usage

```lua
local parser    = require("coding_adventures.wasm_module_parser")
local simulator = require("coding_adventures.wasm_simulator")

-- Load and parse a .wasm file
local f = io.open("add.wasm", "rb")
local wasm_bytes = f:read("*all")
f:close()

local module   = parser.parse(wasm_bytes)
local instance = simulator.Instance.new(module)

-- Call an exported function
local results = instance:call("add", {3, 4})
print(results[1])  --> 7

-- Read/write linear memory
instance:memory_write(0, {0xFF, 0x00, 0x00, 0x00})
local bytes = instance:memory_read(0, 4)

-- Access global variables
local val = instance:get_global("my_global")
instance:set_global("my_global", 42)
```

## Execution Model

WebAssembly is a **stack machine**. Instructions operate on an implicit value
stack: each instruction pops operands from the stack and pushes results.

```
  i32.const 3   →   stack: [3]
  i32.const 4   →   stack: [3, 4]
  i32.add       →   stack: [7]    (pops 3 and 4, pushes 7)
```

**Linear memory** is a flat byte array organized in 64 KiB pages. The
`memory.grow` instruction can expand it at runtime.

**Globals** are module-level variables initialized from constant expressions
and optionally exported for host access.

**Control flow** uses a label stack for structured `block`/`loop`/`if`
regions. `br` and `br_if` branch to label targets — either the end of a block
or the start of a loop.

## Supported Instructions

| Category   | Instructions |
|------------|-------------|
| Numeric    | `i32.const`, `i32.add`, `i32.sub`, `i32.mul`, `i32.div_s`, `i32.rem_s`, `i32.and`, `i32.or`, `i32.xor`, `i32.shl`, `i32.shr_s` |
| Comparison | `i32.eq`, `i32.ne`, `i32.lt_s`, `i32.le_s`, `i32.gt_s`, `i32.ge_s`, `i32.eqz` |
| Control    | `nop`, `unreachable`, `block`, `loop`, `if`, `else`, `end`, `br`, `br_if`, `return`, `call` |
| Variable   | `local.get`, `local.set`, `local.tee`, `global.get`, `global.set` |
| Memory     | `i32.load`, `i32.store`, `memory.size`, `memory.grow` |
| Stack      | `drop`, `select` |

## API Reference

### `Instance.new(module) → instance`

Create a new simulator instance from a parsed Wasm module. Initializes globals,
allocates memory, and applies data segments.

### `instance:call(func_name, args) → results`

Call an exported function by name. `args` is an array of Lua numbers (may be
nil for zero-argument functions). Returns an array of result values.

### `instance:call_by_index(func_idx, args) → results`

Call a function by its 0-based module index. Used internally by `call`.

### `instance:get_global(name) → value`

Get the current value of an exported global variable.

### `instance:set_global(name, value)`

Set the value of an exported mutable global variable. Errors if the global
is immutable.

### `instance:memory_read(offset, length) → byte_array`

Read `length` bytes from linear memory starting at byte `offset`.
Returns a 1-indexed Lua array of byte values (0–255).

### `instance:memory_write(offset, byte_array)`

Write a 1-indexed array of byte values into linear memory starting at `offset`.

## License

MIT
