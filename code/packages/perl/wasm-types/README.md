# CodingAdventures::WasmTypes (Perl)

WebAssembly value types and fundamental type definitions — the building blocks
for any WebAssembly binary format tool.

## Overview

This module defines all WebAssembly fundamental types from the
[WebAssembly specification](https://webassembly.github.io/spec/core/) and
provides encode/decode functions for their binary representations.

## How It Fits in the Stack

```
logic_gates → arithmetic → wasm_leb128 → wasm_types → wasm_opcodes
```

## Types Defined

### Value Types (%ValType)

| Key        | Value | Description                              |
|------------|-------|------------------------------------------|
| `i32`      | 0x7F  | 32-bit integer                           |
| `i64`      | 0x7E  | 64-bit integer                           |
| `f32`      | 0x7D  | 32-bit IEEE 754 float                    |
| `f64`      | 0x7C  | 64-bit IEEE 754 float                    |
| `v128`     | 0x7B  | 128-bit SIMD vector                      |
| `funcref`  | 0x70  | Opaque function reference (nullable)     |
| `externref`| 0x6F  | Opaque external/host reference (nullable)|

### Reference Types (%RefType)
Subset: `funcref` (0x70) and `externref` (0x6F).

### Block Types (%BlockType)
`empty` = 0x40 (void / epsilon block type).

### External Types (%ExternType)
`func`=0, `table`=1, `mem`=2, `global`=3.

## Functions

| Function                       | Description                                |
|--------------------------------|--------------------------------------------|
| `is_val_type($byte)`           | True if byte is a valid ValType            |
| `is_ref_type($byte)`           | True if byte is funcref or externref       |
| `val_type_name($byte)`         | "i32", "i64", … or "unknown_0xXX"          |
| `encode_val_type($vt)`         | Returns one-element byte list              |
| `decode_val_type($aref, $off)` | Returns (type, bytes_consumed)             |
| `encode_limits(\%lim)`         | Encode Limits as byte list                 |
| `decode_limits($aref, $off)`   | Returns (hashref, bytes_consumed)          |
| `encode_func_type(\%ft)`       | Encode FuncType as byte list               |
| `decode_func_type($aref, $off)`| Returns (hashref, bytes_consumed)          |

## Usage

```perl
use CodingAdventures::WasmTypes qw(
    is_val_type is_ref_type val_type_name
    encode_val_type decode_val_type
    encode_limits   decode_limits
    encode_func_type decode_func_type
);

# Type predicates
is_val_type(0x7F)   # 1    (i32)
is_ref_type(0x70)   # 1    (funcref)
is_val_type(0x60)   # ''   (FuncType magic, not a ValType)

# Human-readable names
val_type_name(0x7E) # "i64"
val_type_name(0x99) # "unknown_0x99"

# Encode/decode individual value types
my @b  = encode_val_type(0x7F);          # (0x7F)
my ($t, $n) = decode_val_type([0x7F]);   # (0x7F, 1)

# Limits (for memory/table sizing)
my @lb = encode_limits({min=>1, max=>4}); # (0x01, 0x01, 0x04)
my ($lim, $lc) = decode_limits(\@lb);     # ({min=>1, max=>4}, 3)

# Function type signatures
my @ft = encode_func_type({params=>[0x7F, 0x7F], results=>[0x7E]});
# (0x60, 0x02, 0x7F, 0x7F, 0x01, 0x7E)

my ($dec, $dc) = decode_func_type(\@ft);
# ({params=>[0x7F,0x7F], results=>[0x7E]}, 6)
```

## Version

0.01
