# coding-adventures-wasm-types (Lua)

WebAssembly value types and fundamental type definitions — the building blocks
for any WebAssembly binary format tool.

## Overview

WebAssembly is a statically typed binary instruction format. This package
defines all the fundamental types from the [WebAssembly specification](https://webassembly.github.io/spec/core/) and provides encode/decode functions
for their binary representations.

## How It Fits in the Stack

```
logic_gates → arithmetic → wasm_leb128 → wasm_types → wasm_opcodes
```

The `wasm_leb128` package provides variable-length integer encoding. This
package uses LEB128 to encode/decode type structures.

## Types Defined

### Value Types (ValType)

| Name       | Byte | Description                              |
|------------|------|------------------------------------------|
| `i32`      | 0x7F | 32-bit integer                           |
| `i64`      | 0x7E | 64-bit integer                           |
| `f32`      | 0x7D | 32-bit IEEE 754 float                    |
| `f64`      | 0x7C | 64-bit IEEE 754 float                    |
| `v128`     | 0x7B | 128-bit SIMD vector                      |
| `funcref`  | 0x70 | Opaque function reference (nullable)     |
| `externref`| 0x6F | Opaque external/host reference (nullable)|

### Reference Types (RefType)
Subset of ValType: `funcref` (0x70) and `externref` (0x6F).

### Block Type
`BlockType.empty` = 0x40 (void / epsilon).

### ExternType
Used in import/export sections: `func`=0, `table`=1, `mem`=2, `global`=3.

## Functions

| Function              | Description                                        |
|-----------------------|----------------------------------------------------|
| `is_val_type(byte)`   | Returns true if byte is a valid ValType            |
| `is_ref_type(byte)`   | Returns true if byte is funcref or externref       |
| `val_type_name(byte)` | Human-readable name, e.g. "i32" or "unknown_0x42" |
| `encode_val_type(vt)` | Encode a ValType as a 1-byte array                 |
| `decode_val_type(b,o)`| Decode a ValType from byte array at offset         |
| `encode_limits(lim)`  | Encode a Limits struct (min/max) as bytes          |
| `decode_limits(b,o)`  | Decode a Limits struct from byte array at offset   |
| `encode_func_type(ft)`| Encode a FuncType (0x60 + params + results)        |
| `decode_func_type(b,o)`| Decode a FuncType from byte array at offset       |

## Usage

```lua
local wt = require("coding_adventures.wasm_types")

-- Type predicates
wt.is_val_type(0x7F)   -- true  (i32)
wt.is_val_type(0x60)   -- false (FuncType magic, not a ValType)
wt.is_ref_type(0x70)   -- true  (funcref)

-- Human-readable names
wt.val_type_name(0x7E) -- "i64"
wt.val_type_name(0x99) -- "unknown_0x99"

-- Encode/decode individual value types
local bytes = wt.encode_val_type(wt.ValType.i32)   -- {0x7F}
local info  = wt.decode_val_type({0x7F, 0x7E}, 1)  -- {type=0x7F, bytes_consumed=1}

-- Limits (memory/table sizes)
wt.encode_limits({min=0})        -- {0x00, 0x00}
wt.encode_limits({min=1, max=4}) -- {0x01, 0x01, 0x04}
wt.decode_limits({0x01, 0x01, 0x10})
-- → {limits={min=1, max=16}, bytes_consumed=3}

-- Function type signatures
local ft = {params={wt.ValType.i32, wt.ValType.i32}, results={wt.ValType.i64}}
wt.encode_func_type(ft)
-- → {0x60, 0x02, 0x7F, 0x7F, 0x01, 0x7E}

wt.decode_func_type({0x60, 0x02, 0x7F, 0x7F, 0x01, 0x7E})
-- → {func_type={params={0x7F,0x7F}, results={0x7E}}, bytes_consumed=6}
```

## Version

0.1.0
