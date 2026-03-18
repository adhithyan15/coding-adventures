# 04f — CLR Simulator

## Overview

A simulator for the Common Language Runtime (CLR) Intermediate Language (IL),
also known as CIL or MSIL. The CLR (2002) is Microsoft's answer to the JVM —
it runs C#, F#, VB.NET, and PowerShell. Like the JVM, it is a **stack-based**
virtual machine, but with notable design differences.

## Where it fits

```
Source → Lexer → Parser → CLR Compiler → CLR Simulator
                              (new)          (new)
```

## Architecture: Stack-based with unified type system

The CLR uses a simpler opcode scheme than the JVM. Where the JVM has
`iadd`, `ladd`, `fadd`, `dadd` (one per type), the CLR has just `add` —
it infers the type from what's on the stack. This makes CLR bytecode
more compact.

```
JVM:    iconst_1 / iconst_2 / iadd     ← type in the opcode
CLR:    ldc.i4.1 / ldc.i4.2 / add      ← type inferred from stack
```

## Minimal instruction set for x = 1 + 2

```
ldc.i4.1          Push int32 constant 1
ldc.i4.2          Push int32 constant 2
add               Pop two, push sum (type inferred)
stloc.0           Store in local variable 0
ret               Return
```

## MVP instruction set

| Opcode | Hex | Description |
|--------|-----|-------------|
| `nop` | 0x00 | No operation |
| `ldnull` | 0x01 | Push null |
| `ldc.i4.0` | 0x16 | Push int32 0 |
| `ldc.i4.1` | 0x17 | Push int32 1 |
| `ldc.i4.2` | 0x18 | Push int32 2 |
| `ldc.i4.3` | 0x19 | Push int32 3 |
| `ldc.i4.4` | 0x1A | Push int32 4 |
| `ldc.i4.5` | 0x1B | Push int32 5 |
| `ldc.i4.s` | 0x1F | Push int8 as int32 (1 operand) |
| `ldc.i4` | 0x20 | Push int32 (4 byte operand) |
| `ldloc.0` | 0x06 | Load local variable 0 |
| `ldloc.1` | 0x07 | Load local variable 1 |
| `ldloc.2` | 0x08 | Load local variable 2 |
| `ldloc.3` | 0x09 | Load local variable 3 |
| `stloc.0` | 0x0A | Store to local variable 0 |
| `stloc.1` | 0x0B | Store to local variable 1 |
| `stloc.2` | 0x0C | Store to local variable 2 |
| `stloc.3` | 0x0D | Store to local variable 3 |
| `ldloc.s` | 0x11 | Load local variable N (1 operand) |
| `stloc.s` | 0x13 | Store to local variable N (1 operand) |
| `add` | 0x58 | Add (type inferred) |
| `sub` | 0x59 | Subtract |
| `mul` | 0x5A | Multiply |
| `div` | 0x5B | Divide |
| `ceq` | 0xFE01 | Compare equal (push 1 or 0) |
| `cgt` | 0xFE02 | Compare greater than |
| `clt` | 0xFE04 | Compare less than |
| `br` | 0x38 | Unconditional branch (4-byte offset) |
| `br.s` | 0x2B | Short branch (1-byte offset) |
| `brfalse.s` | 0x2C | Branch if false/zero (short) |
| `brtrue.s` | 0x2D | Branch if true/nonzero (short) |
| `ret` | 0x2A | Return |
| `call` | 0x28 | Call method |

## Key differences from JVM

| Feature | JVM | CLR |
|---------|-----|-----|
| Type in opcodes | Yes (`iadd`, `ladd`) | No (`add` — inferred) |
| Generics | Type erasure (fake at runtime) | Reified (real at runtime) |
| Value types | No (everything is object) | Yes (`struct` on stack) |
| Multi-byte opcodes | No | Yes (`0xFE` prefix for `ceq`, `cgt`, `clt`) |
| Short encodings | `iconst_0`..`iconst_5` | `ldc.i4.0`..`ldc.i4.8` (more!) |

## Implementation notes

- Standalone simulator (does not wrap generic CPU)
- Bytecode is a `bytes` object (variable-width)
- Multi-byte opcodes: `ceq` = `0xFE 0x01` (2 bytes for opcode)
- Step trace captures: PC, opcode, stack before/after, locals snapshot
- Use real CLR opcode values for educational accuracy
