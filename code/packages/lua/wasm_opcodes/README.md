# coding-adventures-wasm-opcodes (Lua)

WebAssembly opcode definitions — a reference table mapping opcode byte values
to human-readable names and operand descriptions.

## Overview

This package provides a complete lookup table for WebAssembly MVP opcodes
(0x00–0xFF single-byte encoding). It is designed as a reference layer for
higher-level tools such as disassemblers, assemblers, validators, and JIT
compilers.

## How It Fits in the Stack

```
logic_gates → arithmetic → wasm_leb128 → wasm_types → wasm_opcodes
```

## Opcode Categories Covered

| Category       | Examples                                          |
|----------------|---------------------------------------------------|
| Control flow   | unreachable, nop, block, loop, if, br, call       |
| Parametric     | drop, select                                      |
| Variable       | local.get, local.set, local.tee, global.get/set   |
| Memory loads   | i32.load, i64.load, f32.load, i32.load8_s, …      |
| Memory stores  | i32.store, i32.store8, i32.store16                |
| Memory size    | memory.size, memory.grow                          |
| i32 numeric    | i32.const, i32.add, i32.sub, i32.mul, i32.and, …  |
| i64 numeric    | i64.const, i64.add, i64.sub, i64.mul              |
| f32 numeric    | f32.const, f32.add, f32.sub, f32.mul              |
| f64 numeric    | f64.const, f64.add, f64.sub, f64.mul              |
| Conversions    | i32.wrap_i64, i64.extend_i32_s, f32.demote_f64, … |

## Functions

| Function               | Description                                          |
|------------------------|------------------------------------------------------|
| `opcode_name(byte)`    | Returns mnemonic string or "unknown_0xXX"            |
| `is_valid_opcode(byte)`| Returns true if byte is a recognized opcode          |
| `get_opcode_info(byte)`| Returns `{name, operands}` table or nil              |

## Usage

```lua
local op = require("coding_adventures.wasm_opcodes")

-- Look up a name
op.opcode_name(0x6a)   -- "i32.add"
op.opcode_name(0x00)   -- "unreachable"
op.opcode_name(0x99)   -- "unknown_0x99"

-- Validate an opcode
op.is_valid_opcode(0x10)  -- true  (call)
op.is_valid_opcode(0x99)  -- false

-- Get full info
local info = op.get_opcode_info(0x28)
-- {name="i32.load", operands="memarg(align:u32, offset:u32)"}

-- Iterate all opcodes
for byte, info in pairs(op.OPCODES) do
    print(string.format("0x%02x  %-20s  %s", byte, info.name, info.operands))
end
```

## OPCODES Table Format

Each entry in `OPCODES` is a table with two string fields:

- `name` — the standard WebAssembly mnemonic (e.g., `"i32.add"`)
- `operands` — a description of the immediate bytes that follow the opcode

Operand notation:
- `"none"` — no immediates
- `"blocktype"` — one block type byte (0x40 or a ValType)
- `"label:u32"` — label index as unsigned LEB128
- `"func_idx:u32"` — function index as unsigned LEB128
- `"memarg(align:u32, offset:u32)"` — two unsigned LEB128 values

## Version

0.1.0
