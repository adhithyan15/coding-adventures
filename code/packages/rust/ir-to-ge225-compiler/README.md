# ir-to-ge225-compiler

Translates a target-independent `IrProgram` into a flat binary image of
GE-225 20-bit machine words.

## What is the GE-225?

The General Electric GE-225 is the 1960-era mainframe that ran Dartmouth's
pioneering BASIC time-sharing system in 1964. It is a **word-addressed
accumulator machine**: memory holds 20-bit signed integers addressed from 0;
the sole arithmetic register is the **Accumulator (A)**; computation routes
through it via a strict load-compute-store rhythm.

## Architecture

Every IR virtual register maps to a *spill slot* — a dedicated memory word in
the data segment. A three-register ADD lowered to GE-225 looks like:

```
LDA [vA]      ; A = spill[vA]
ADD [vB]      ; A = A + spill[vB]
STA [vDst]    ; spill[vDst] = A
```

### Memory layout

```
┌──────────────────────────────────────────┐
│ addr 0           : TON (typewriter on)   │
│ addr 1 … code_end-1 : IR code words      │
│ addr code_end    : BRU code_end (halt)   │
│ addr data_base …: spill slots (v0…vN)    │
│ addr …           : constants table       │
└──────────────────────────────────────────┘
```

## Usage

```rust
use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};
use ir_to_ge225_compiler::{compile_to_ge225, validate_for_ge225};

let mut prog = IrProgram::new("_start");
prog.add_instruction(IrInstruction::new(
    IrOp::LoadImm,
    vec![IrOperand::Register(0), IrOperand::Immediate(42)],
    1,
));
prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 2));

let errors = validate_for_ge225(&prog);
assert!(errors.is_empty());

let result = compile_to_ge225(&prog).unwrap();
println!("halt address: {}", result.halt_address);
println!("binary size: {} bytes", result.binary.len());
```

## Supported opcodes

| IR Opcode | GE-225 sequence | Words |
|-----------|-----------------|-------|
| `LABEL` | — | 0 |
| `COMMENT` | — | 0 |
| `NOP` | `NOP` | 1 |
| `HALT` | `BRU code_end` | 1 |
| `JUMP` | `BRU target` | 1 |
| `LOAD_IMM` | `LDA const; STA spill` | 2 |
| `ADD_IMM 0` | `LDA src; STA dst` (copy) | 2 |
| `ADD_IMM ±1` | `LDA src; ADO/SBO; STA dst` | 3 |
| `ADD_IMM n` | `LDA src; ADD const; STA dst` | 3 |
| `ADD` / `SUB` | `LDA a; ADD/SUB b; STA dst` | 3 |
| `BRANCH_Z` / `BRANCH_NZ` | `LDA reg; BNZ/BZE; BRU target` | 3 |
| `SYSCALL 1` | `LDA v0; SAN 6; TYP` | 3 |
| `DIV` | `LDA a; LQA; LDZ; DVD b; STA dst` | 5 |
| `MUL` | `LDA a; LQA; LDZ; MPY b; LAQ; STA dst` | 6 |
| `AND_IMM 1` | BOD-branch odd-bit extraction | 7 |
| `CMP_EQ` / `CMP_NE` / `CMP_LT` / `CMP_GT` | subtract + conditional skip | 8 |

## How it fits in the stack

```
Frontend (Brainfuck, Nib, Tetrad, ...)
    ↓ IrProgram
[ir-to-ge225-compiler]  ← this crate
    ↓ CompileResult (binary: Vec<u8>)
[ge225-simulator] → run on the 1964 mainframe
```

## Crate structure

- `validate_for_ge225(program)` — pre-flight validation, returns `Vec<String>`
- `compile_to_ge225(program)` → `Result<CompileResult, CodeGenError>`
- `CompileResult` — `binary`, `halt_address`, `data_base`, `label_map`
- `CodeGenError` — error from unsupported opcodes, bad immediates, undefined labels
- `GE225CodeGenerator` — implements the `CodeGenerator<IrProgram, CompileResult>` protocol
