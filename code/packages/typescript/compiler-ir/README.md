# @coding-adventures/compiler-ir

Intermediate Representation (IR) types, opcodes, printer, and parser for the AOT compiler pipeline. TypeScript port of `code/packages/go/compiler-ir`.

## What is IR?

An IR is a language-independent representation of computation. Think of it as assembly for an imaginary, idealized computer with infinite registers and no platform quirks.

The compiler pipeline:

```
Source code (Brainfuck)
    ↓  frontend (brainfuck-ir-compiler)
IR instructions
    ↓  optimizer (compiler-ir-optimizer)
Optimized IR instructions
    ↓  backend (codegen-riscv)
Machine code (RISC-V .text section)
```

This package defines the IR layer: the data structures that flow between stages, plus utilities for printing and parsing IR text.

## Usage

```typescript
import {
  IrProgram, IDGenerator,
  IrOp, reg, imm, lbl,
  printIr, parseIr
} from "@coding-adventures/compiler-ir";

// Build a program
const prog = new IrProgram("_start");
prog.addData({ label: "tape", size: 30000, init: 0 });

const gen = new IDGenerator();
prog.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_start")], id: -1 });
prog.addInstruction({ opcode: IrOp.LOAD_ADDR, operands: [reg(0), lbl("tape")], id: gen.next() });
prog.addInstruction({ opcode: IrOp.HALT, operands: [], id: gen.next() });

// Print to text
const text = printIr(prog);
console.log(text);
// .version 1
//
// .data tape 30000 0
//
// .entry _start
//
// _start:
//   LOAD_ADDR   v0, tape  ; #0
//   HALT                  ; #1

// Parse back from text (roundtrip)
const reparsed = parseIr(text);
```

## Opcodes

25 opcodes in v1 (Brainfuck-sufficient subset):

| Group       | Opcodes |
|-------------|---------|
| Constants   | LOAD_IMM, LOAD_ADDR |
| Memory      | LOAD_BYTE, STORE_BYTE, LOAD_WORD, STORE_WORD |
| Arithmetic  | ADD, ADD_IMM, SUB, AND, AND_IMM |
| Comparison  | CMP_EQ, CMP_NE, CMP_LT, CMP_GT |
| Control Flow | LABEL, JUMP, BRANCH_Z, BRANCH_NZ, CALL, RET |
| System      | SYSCALL, HALT |
| Meta        | NOP, COMMENT |

## Operand types

```typescript
reg(0)          // IrRegister: v0
imm(42)         // IrImmediate: 42
lbl("_start")   // IrLabel: _start
```

## Text format

```
.version 1

.data tape 30000 0

.entry _start

_start:
  LOAD_ADDR   v0, tape  ; #0
  LOAD_IMM    v1, 0     ; #1
  HALT                  ; #2
```

## Stack position

Layer 5 — Compiler Infrastructure. Sits between the language frontends (brainfuck-ir-compiler) and the backends (codegen-riscv).
