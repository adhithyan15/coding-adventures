# coding_adventures_compiler_ir

General-purpose intermediate representation (IR) type library for the AOT compiler pipeline.

## What it does

This gem defines the IR used between language frontends (e.g., Brainfuck, BASIC) and machine code backends (e.g., RISC-V, x86). The IR is:

- **Linear**: no basic blocks, no SSA, no phi nodes — just a flat sequence of instructions
- **Register-based**: infinite virtual registers (v0, v1, ...) — the backend allocates physical registers
- **Target-independent**: no platform assumptions baked in
- **Versioned**: `.version 1` covers the Brainfuck subset; new opcodes are appended as new languages are added

## How it fits in the stack

```
Source code
    ↓  (brainfuck-ir-compiler)
IrProgram           ← this gem
    ↓  (compiler-ir-optimizer)
Optimised IrProgram
    ↓  (codegen-riscv)
ELF binary
```

## Key types

| Type | Description |
|------|-------------|
| `IrOp` | 25 opcode constants (0–24) with name lookup and parse |
| `IrRegister` | Virtual register: `v0`, `v5`, ... |
| `IrImmediate` | Integer literal: `42`, `-1`, ... |
| `IrLabel` | Named target: `_start`, `loop_0_end`, ... |
| `IrInstruction` | Opcode + operands + unique ID |
| `IrDataDecl` | `.data tape 30000 0` — named memory region |
| `IrProgram` | Complete program: instructions + data + entry label |
| `IDGenerator` | Monotonic ID counter for instruction source mapping |
| `IrPrinter` | `IrProgram → String` (canonical text format) |
| `IrParser` | `String → IrProgram` (inverse of IrPrinter) |

## Usage

```ruby
require "coding_adventures_compiler_ir"

include CodingAdventures::CompilerIr

# Build a program by hand
prog = IrProgram.new("_start")
gen  = IDGenerator.new

prog.add_data(IrDataDecl.new("tape", 30_000, 0))
prog.add_instruction(IrInstruction.new(IrOp::LABEL,    [IrLabel.new("_start")],  -1))
prog.add_instruction(IrInstruction.new(IrOp::LOAD_ADDR, [IrRegister.new(0), IrLabel.new("tape")], gen.next))
prog.add_instruction(IrInstruction.new(IrOp::HALT,     [], gen.next))

# Print to canonical text
text = IrPrinter.print(prog)
puts text
# .version 1
#
# .data tape 30000 0
#
# .entry _start
#
# _start:
#   LOAD_ADDR   v0, tape  ; #0
#   HALT                  ; #1

# Parse back
parsed = IrParser.parse(text)
parsed.instructions.length  #=> 2 (label + halt)
```

## Opcode reference

| Opcode | Value | Category | Semantics |
|--------|-------|----------|-----------|
| LOAD_IMM | 0 | Constants | `dst = imm` |
| LOAD_ADDR | 1 | Constants | `dst = &label` |
| LOAD_BYTE | 2 | Memory | `dst = mem[base+off] & 0xFF` |
| STORE_BYTE | 3 | Memory | `mem[base+off] = src & 0xFF` |
| LOAD_WORD | 4 | Memory | `dst = *(word*)(base+off)` |
| STORE_WORD | 5 | Memory | `*(word*)(base+off) = src` |
| ADD | 6 | Arithmetic | `dst = lhs + rhs` |
| ADD_IMM | 7 | Arithmetic | `dst = src + imm` |
| SUB | 8 | Arithmetic | `dst = lhs - rhs` |
| AND | 9 | Arithmetic | `dst = lhs & rhs` |
| AND_IMM | 10 | Arithmetic | `dst = src & imm` |
| CMP_EQ | 11 | Comparison | `dst = lhs == rhs ? 1 : 0` |
| CMP_NE | 12 | Comparison | `dst = lhs != rhs ? 1 : 0` |
| CMP_LT | 13 | Comparison | `dst = lhs < rhs ? 1 : 0` |
| CMP_GT | 14 | Comparison | `dst = lhs > rhs ? 1 : 0` |
| LABEL | 15 | Control Flow | Label definition (no machine code) |
| JUMP | 16 | Control Flow | `PC = &label` |
| BRANCH_Z | 17 | Control Flow | `if reg == 0: PC = &label` |
| BRANCH_NZ | 18 | Control Flow | `if reg != 0: PC = &label` |
| CALL | 19 | Control Flow | Call subroutine |
| RET | 20 | Control Flow | Return from subroutine |
| SYSCALL | 21 | System | Platform syscall |
| HALT | 22 | System | Terminate program |
| NOP | 23 | Meta | No operation |
| COMMENT | 24 | Meta | Human-readable annotation (no machine code) |
