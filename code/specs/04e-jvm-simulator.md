# 04e — JVM Simulator

## Overview

A simulator for the Java Virtual Machine (JVM) bytecode instruction set.
The JVM (1995) is the most widely deployed virtual machine in history —
it runs Java, Kotlin, Scala, Clojure, and Groovy. It is a **stack-based**
machine, like our VM and WASM, but with a much richer type system.

## Where it fits

```
Source → Lexer → Parser → JVM Compiler → JVM Simulator
                              (new)          (new)
```

The JVM simulator sits at the same layer as WASM, ARM, and RISC-V — it's
another execution target. The JVM compiler translates our AST into JVM
bytecode, and the simulator executes it.

## Architecture: Stack-based with typed opcodes

Unlike our VM (which has a single ADD instruction), the JVM has separate
opcodes for each type:

```
Our VM:    ADD           ← works on whatever's on the stack
JVM:       iadd          ← integer add
           ladd          ← long add
           fadd          ← float add
           dadd          ← double add
```

The JVM also has a **local variable array** (like our STORE_LOCAL/LOAD_LOCAL)
alongside the operand stack.

## Minimal instruction set for x = 1 + 2

```
iconst_1          Push integer constant 1
iconst_2          Push integer constant 2
iadd              Pop two integers, push their sum
istore_0          Pop and store in local variable 0
return            Return from method
```

## MVP instruction set

| Opcode | Hex | Description |
|--------|-----|-------------|
| `iconst_0` | 0x03 | Push int 0 |
| `iconst_1` | 0x04 | Push int 1 |
| `iconst_2` | 0x05 | Push int 2 |
| `iconst_3` | 0x06 | Push int 3 |
| `iconst_4` | 0x07 | Push int 4 |
| `iconst_5` | 0x08 | Push int 5 |
| `bipush` | 0x10 | Push byte as int (1 operand) |
| `sipush` | 0x11 | Push short as int (2 operands) |
| `ldc` | 0x12 | Push from constant pool (1 operand) |
| `iload` | 0x15 | Load int from local variable |
| `iload_0` | 0x1A | Load int from local 0 |
| `iload_1` | 0x1B | Load int from local 1 |
| `iload_2` | 0x1C | Load int from local 2 |
| `iload_3` | 0x1D | Load int from local 3 |
| `istore` | 0x36 | Store int to local variable |
| `istore_0` | 0x3B | Store int to local 0 |
| `istore_1` | 0x3C | Store int to local 1 |
| `istore_2` | 0x3D | Store int to local 2 |
| `istore_3` | 0x3E | Store int to local 3 |
| `iadd` | 0x60 | Add two ints |
| `isub` | 0x64 | Subtract two ints |
| `imul` | 0x68 | Multiply two ints |
| `idiv` | 0x6C | Divide two ints |
| `if_icmpeq` | 0x9F | Branch if ints equal |
| `if_icmpgt` | 0xA3 | Branch if int > int |
| `goto` | 0xA7 | Unconditional branch |
| `ireturn` | 0xAC | Return int from method |
| `return` | 0xB1 | Return void |
| `getstatic` | 0xB2 | Get static field (for System.out) |
| `invokevirtual` | 0xB6 | Invoke method (for println) |

## Key differences from our VM

| Feature | Our VM | JVM |
|---------|--------|-----|
| Types | Untyped stack | Typed opcodes (i/l/f/d prefix) |
| Variables | Named (hash map) | Numbered slots (array) |
| Constants | Index into pool | `iconst_N` shortcuts + pool |
| Encoding | Each instruction is an object | Variable-width bytes |
| Methods | Simple CALL/RETURN | Full method descriptors |

## Implementation notes

- Standalone simulator (does not wrap the generic CPU)
- Bytecode is a `bytes` object (variable-width encoding)
- Step trace captures: PC, opcode, stack before/after, locals snapshot
- Use real JVM opcode values for educational accuracy
