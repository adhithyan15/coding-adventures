# arm-simulator

ARMv7 educational simulator -- conditional execution, NZCV, memory, and branches.

## What is this?

This crate contains two compatible lanes:

- `ARMSimulator` preserves the original three-instruction wrapper built on
  `cpu-simulator` for existing consumers.
- `functional::Armv7Simulator` is the completed Spec 07b machine. It owns an
  exact 64 KiB little-endian memory, 16 32-bit registers with R15 mirrored to
  PC, NZCV flags, halt state, and installed-program metadata. Its checked
  restore/load/direct-access/step/run API uses typed errors and atomic rollback.

## Supported Instructions

| Instruction      | Description                       |
|------------------|-----------------------------------|
| `MOV`                 | Move rotated immediate or register |
| `ADD`, `SUB`, `CMP`   | Arithmetic and comparison with NZCV |
| `AND`, `ORR`          | Boolean data processing            |
| `LDR`, `STR`          | Pre-indexed word memory access      |
| `B`, `BL`, `BEQ/BNE`  | PC+8-relative conditional branch   |
| `HLT`                 | Halt (custom sentinel)              |

All sixteen ARM condition predicates are implemented. The documented S bit
updates NZCV, while CMP always updates flags and writes no register.

## How it fits in the stack

The legacy lane builds on `cpu-simulator`. The completed functional lane is a
self-contained oracle suitable for later gate-level differential testing.

## Usage

```rust
use arm_simulator::*;

let mut sim = ARMSimulator::new(65536);
let program = assemble(&[
    encode_mov_imm(0, 10),  // R0 = 10
    encode_mov_imm(1, 3),   // R1 = 3
    encode_sub(2, 0, 1),    // R2 = R0 - R1 = 7
    encode_hlt(),
]);
let traces = sim.run(&program);
assert_eq!(sim.cpu.registers.read(2), 7);
```

For the checked completion contract:

```rust
use arm_simulator::functional::{
    Armv7Simulator, Condition, DataOpcode, HALT_WORD, assemble_words,
    encode_data_immediate,
};

let program = assemble_words(&[
    encode_data_immediate(Condition::Al, DataOpcode::Mov, false, 0, 0, 0, 42),
    HALT_WORD,
]);
let result = Armv7Simulator::new().run_checked(&program, 4)?;
assert_eq!(result.final_state.registers[0], 42);
# Ok::<(), arm_simulator::functional::ArmError>(())
```

## Verification

The package includes lifecycle, Spec 07b, and 388-vector reproducible Python
common-surface full-state differential suites. Package line coverage is 90.15%
(714/792 lines) under `cargo llvm-cov`, above the 80% completion floor.
