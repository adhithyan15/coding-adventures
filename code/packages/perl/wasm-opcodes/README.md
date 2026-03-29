# CodingAdventures::WasmOpcodes (Perl)

WebAssembly opcode definitions — a reference table mapping opcode byte values
to human-readable names and operand descriptions.

## Overview

This module provides a complete `%OPCODES` hash for WebAssembly MVP opcodes
(single-byte 0x00–0xFF range). It serves as the reference layer for
disassemblers, assemblers, validators, and JIT compilers.

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
| i32 numeric    | i32.const, eqz, eq, ne, lt_s, add, sub, mul, …   |
| i64 numeric    | i64.const, add, sub, mul                          |
| f32 numeric    | f32.const, add, sub, mul                          |
| f64 numeric    | f64.const, add, sub, mul                          |
| Conversions    | i32.wrap_i64, i64.extend_i32_s, f32.demote_f64, …|

## Functions

| Function               | Description                                          |
|------------------------|------------------------------------------------------|
| `opcode_name($byte)`   | Returns mnemonic string or "unknown_0xXX"            |
| `is_valid_opcode($byte)`| Returns 1 if byte is a recognized opcode, '' if not |
| `get_opcode_info($byte)`| Returns hashref {name, operands} or undef           |

## Usage

```perl
use CodingAdventures::WasmOpcodes qw(opcode_name is_valid_opcode get_opcode_info);

# Look up names
opcode_name(0x6a)   # "i32.add"
opcode_name(0x00)   # "unreachable"
opcode_name(0x99)   # "unknown_0x99"

# Validate
is_valid_opcode(0x10)  # 1  (call)
is_valid_opcode(0x99)  # '' (unknown)

# Get full info
my $info = get_opcode_info(0x28);
# { name => "i32.load", operands => "memarg(align:u32,offset:u32)" }

# Iterate all opcodes
for my $byte (sort { $a <=> $b } keys %CodingAdventures::WasmOpcodes::OPCODES) {
    my $e = $CodingAdventures::WasmOpcodes::OPCODES{$byte};
    printf "0x%02x  %-20s  %s\n", $byte, $e->{name}, $e->{operands};
}
```

## %OPCODES Table Format

Keys are integer byte values. Values are hashrefs with:

- `name` — standard WebAssembly mnemonic (e.g., `"i32.add"`)
- `operands` — description of immediate bytes following the opcode

## Version

0.01
