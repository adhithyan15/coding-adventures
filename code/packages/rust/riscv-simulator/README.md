# RISC-V Simulator (Rust)

Full RV32I base integer instruction set, the `mul` / `mulhu` / `div` / `divu`
/ `rem` / `remu` RV32M subset,
and M-mode privileged extensions.

The package has two intentionally separate entry points:

- `Rv32ISimulator` is the normative Spec 07a machine. It owns exact 64 KiB
  state, validates restore/load/direct access, reports typed decode/alignment/
  range/lifecycle faults, and provides transition-atomic steps plus
  transactional bounded runs with complete before/after traces.
- `RiscVSimulator` is the preserved compiler-facing compatibility harness. It
  additionally exposes the selected RV32M operations and repository host
  services used by existing backends.

## Supported Instructions

- **Arithmetic**: add, sub, addi, slt, sltu, slti, sltiu, and, or, xor, andi, ori, xori
- **Multiply/divide**: mul, mulhu, div, divu, rem, remu (RV32M subset)
- **Shifts**: sll, srl, sra, slli, srli, srai
- **Loads**: lb, lh, lw, lbu, lhu
- **Stores**: sb, sh, sw
- **Branches**: beq, bne, blt, bge, bltu, bgeu
- **Jumps**: jal, jalr
- **Upper immediates**: lui, auipc
- **System**: ecall, mret, csrrw, csrrs, csrrc
- **CSR registers**: mstatus, mtvec, mepc, mcause, mscratch

## Architecture

```
opcodes.rs   -- opcode and funct3/funct7 constants
decode.rs    -- instruction decoder for all six formats (R/I/S/B/U/J)
execute.rs   -- instruction executor for all operations
csr.rs       -- Control and Status Register file for M-mode
encoding.rs  -- helpers to construct machine code for testing
simulator.rs -- top-level simulator with fetch-decode-execute loop
functional.rs -- exact checked RV32I lifecycle and architectural state
```

## Usage

```rust
use riscv_simulator::RiscVSimulator;
use riscv_simulator::encoding::*;

let mut sim = RiscVSimulator::new(65536);
sim.run_instructions(&[
    encode_addi(1, 0, 1),   // x1 = 1
    encode_addi(2, 0, 2),   // x2 = 2
    encode_add(3, 1, 2),    // x3 = 3
    encode_ecall(),          // halt
]);
assert_eq!(sim.regs.read(3), 3);
```

For new architectural tests, prefer the checked machine:

```rust
use riscv_simulator::{encoding, Rv32ISimulator};

let program = encoding::assemble(&[
    encoding::encode_addi(1, 0, 1),
    encoding::encode_ecall(),
]);
let mut sim = Rv32ISimulator::new();
sim.load_at_checked(&program, 0x100).unwrap();
let result = sim.run_checked(2).unwrap();
assert_eq!(result.final_state.registers[1], 1);
```

The checked lifecycle is covered by seven focused suites and a reproducible
256-case Python one-step full-state corpus. All 106 Rust test functions pass;
strict Rustfmt, Clippy, and rustdoc are green. Package line coverage is 91.46%
(1,125/1,230).
