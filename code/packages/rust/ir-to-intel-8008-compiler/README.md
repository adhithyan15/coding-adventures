# ir-to-intel-8008-compiler

Translates a target-independent `IrProgram` into Intel 8008 assembly text.

## What is the Intel 8008?

The Intel 8008 (1972) is the world's first single-chip 8-bit microprocessor —
the ancestor of the x86 family. It has a small but capable register file:

- **A** — Accumulator: 8-bit implicit result register for all ALU ops.
- **B, C, D, E** — Four 8-bit general-purpose registers.
- **H, L** — High/low bytes of the 14-bit memory address register.
- **M** — Pseudo-register: the memory byte at address H:L.

## Physical register assignment

Virtual IR registers map to 8008 hardware registers:

| IR register | Physical | Role |
|-------------|----------|------|
| v0 | B | constant zero, preloaded to 0 at `_start` |
| v1 | C | scratch / return value |
| v2 | D | 1st local / 1st argument |
| v3 | E | 2nd local / 2nd argument |
| v4 | H | 3rd local (careful: H is also memory high byte) |
| v5 | L | 4th local (careful: L is also memory low byte) |

## Dangerous opcode conflicts

Three `MOV A, {reg}` encodings are occupied by other instructions:

- `MOV A, C` = 0x79 → **IN 7** (reads input port 7!)
- `MOV A, H` = 0x7C → **JMP** (unconditional 3-byte jump!)
- `MOV A, M` = 0x7E → **CAL** (subroutine call!)

The compiler uses the safe group-10 ALU path instead:
```asm
MVI  A, 0     ; prime accumulator
ADD  C        ; A = 0 + C = C  (group-10, always register read)
```

## Usage

```rust
use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};
use ir_to_intel_8008_compiler::{IrToIntel8008Compiler, IrValidator};

let mut prog = IrProgram::new("_start");
prog.add_instruction(IrInstruction::new(
    IrOp::LoadImm,
    vec![IrOperand::Register(1), IrOperand::Immediate(42)],
    1,
));
prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 2));

let validator = IrValidator;
let errors = validator.validate(&prog);
assert!(errors.is_empty());

let compiler = IrToIntel8008Compiler;
let asm = compiler.compile(&prog).unwrap();
println!("{}", asm);
// Output:
//     ORG 0x0000
//     MVI  C, 42
//     HLT
```

## Validation rules

| Rule | Constraint |
|------|------------|
| no_word_ops | `LOAD_WORD` and `STORE_WORD` are forbidden |
| register_count | At most 6 distinct virtual register indices (v0–v5) |
| imm_range | `LOAD_IMM` / `ADD_IMM` immediates must fit in `u8` (0–255) |
| syscall_whitelist | SYSCALL numbers ∈ {3,4} ∪ {11–16} ∪ {20–27} ∪ {40–63} |
| static_data | Sum of all data declarations ≤ 8 191 bytes |

## How it fits in the stack

```
Frontend (Brainfuck, Nib, Oct, ...)
    ↓ IrProgram
[ir-to-intel-8008-compiler]  ← this crate
    ↓ String (Intel 8008 assembly text)
[intel-8008-assembler] → bytes
[intel8008-simulator]  → run on the 1972 microprocessor
```
