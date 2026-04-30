# compiler-ir

General-purpose intermediate representation (IR) type library for the AOT compiler pipeline.

## What it does

This package provides the IR data structures used by the AOT native compiler pipeline:

- **Opcodes** (`IrOp`) — stable integer opcodes covering constants, memory,
  arithmetic, bitwise logic, 64-bit floating operations including `F64_SQRT`,
  comparison, control flow, system calls, and meta instructions
- **Operand types** — `IrRegister` (v0, v1, ...), `IrImmediate` (literal integers), `IrLabel` (named jump targets)
- **Instruction** (`IrInstruction`) — opcode + operands + unique ID for source mapping
- **Data declarations** (`IrDataDecl`) — named static memory regions
- **Program** (`IrProgram`) — the complete IR: instructions + data + entry label + version
- **ID generator** (`IDGenerator`) — monotonic unique IDs for source map tracking
- **Printer** (`print_ir`) — convert IrProgram to canonical text format
- **Parser** (`parse_ir`) — convert canonical text back to IrProgram (roundtrip fidelity)

## How it fits in the stack

```
Source code (Brainfuck, BASIC, ...)
    ↓  [frontend compiler]
IrProgram  ←── this package
    ↓  [optimizer passes]
Optimized IrProgram
    ↓  [backend codegen]
Machine code (RISC-V, ARM, ...)
```

## Usage

```python
from compiler_ir import (
    IrProgram, IrInstruction, IrDataDecl,
    IrRegister, IrImmediate, IrLabel,
    IrOp, IDGenerator,
    print_ir, parse_ir,
)

# Build a program
gen = IDGenerator()
prog = IrProgram(entry_label="_start")
prog.add_data(IrDataDecl("tape", 30000, 0))
prog.add_instruction(IrInstruction(
    opcode=IrOp.LOAD_ADDR,
    operands=[IrRegister(0), IrLabel("tape")],
    id=gen.next(),
))
prog.add_instruction(IrInstruction(opcode=IrOp.HALT, id=gen.next()))

# Print to canonical text
text = print_ir(prog)

# Parse back
prog2 = parse_ir(text)
```

## IR Text Format

```
.version 1

.data tape 30000 0

.entry _start

_start:
  LOAD_ADDR   v0, tape          ; #0
  LOAD_IMM    v1, 0             ; #1
  HALT                          ; #2
```
