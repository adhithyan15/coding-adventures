# compiler-ir

General-purpose intermediate representation (IR) for the AOT compiler pipeline.

## Overview

`compiler-ir` is the IR layer of the compiler pipeline. It defines:

- **`IrOp`** — 25 opcodes covering constants, memory, arithmetic, comparison, control flow, system calls, and meta-operations
- **`IrOperand`** — three operand kinds: virtual register (`v0`), immediate (`42`), label (`_start`)
- **`IrInstruction`** — an opcode + operands + unique monotonic ID
- **`IrDataDecl`** — a data segment declaration (label, size, init byte)
- **`IrProgram`** — a complete program (instructions + data + entry label)
- **`IdGenerator`** — produces unique monotonic instruction IDs
- **`print_ir`** — serializes an `IrProgram` to canonical text
- **`parse_ir`** — deserializes canonical text back to an `IrProgram`

## Where it fits

```
Brainfuck source
      |
 [brainfuck-ir-compiler] → IrProgram  ← THIS CRATE defines IrProgram
      |
 [optimizer] (future)
      |
 [codegen-riscv] (future)
```

## IR text format

```
.version 1

.data tape 30000 0

.entry _start

_start:
  LOAD_ADDR   v0, tape          ; #0
  LOAD_IMM    v1, 0             ; #1
  HALT                          ; #2
```

## Usage

```rust
use compiler_ir::types::{IrProgram, IrInstruction, IrDataDecl, IrOperand};
use compiler_ir::opcodes::IrOp;
use compiler_ir::printer::print_ir;
use compiler_ir::ir_parser::parse_ir;

// Build a minimal program
let mut prog = IrProgram::new("_start");
prog.add_data(IrDataDecl { label: "tape".to_string(), size: 30000, init: 0 });
prog.add_instruction(IrInstruction::new(
    IrOp::LoadAddr,
    vec![IrOperand::Register(0), IrOperand::Label("tape".to_string())],
    0,
));
prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));

// Serialize to text
let text = print_ir(&prog);

// Deserialize back
let parsed = parse_ir(&text).unwrap();
assert_eq!(parsed.instructions.len(), prog.instructions.len());
```

## Opcode categories

| Category     | Opcodes |
|--------------|---------|
| Constants    | `LOAD_IMM`, `LOAD_ADDR` |
| Memory       | `LOAD_BYTE`, `STORE_BYTE`, `LOAD_WORD`, `STORE_WORD` |
| Arithmetic   | `ADD`, `ADD_IMM`, `SUB`, `AND`, `AND_IMM` |
| Comparison   | `CMP_EQ`, `CMP_NE`, `CMP_LT`, `CMP_GT` |
| Control Flow | `LABEL`, `JUMP`, `BRANCH_Z`, `BRANCH_NZ`, `CALL`, `RET` |
| System       | `SYSCALL`, `HALT` |
| Meta         | `NOP`, `COMMENT` |

## Design principles

1. **General-purpose** — designed for any compiled language, not just Brainfuck
2. **Versioned** — new opcodes are only ever appended; existing ones never change semantics
3. **Target-independent** — backends map IR opcodes to physical ISAs (RISC-V, ARM, x86-64)
4. **Linear** — no basic blocks, no SSA, no phi nodes
5. **Register-based** — infinite virtual registers mapped to physical registers by the backend
