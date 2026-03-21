# RISC-V Simulator

**Layer 7a of the computing stack** -- implements the full RISC-V RV32I base integer instruction set with M-mode privileged extensions.

## What this package does

Decodes and executes all 37 RV32I instructions plus M-mode privileged operations:

- **Arithmetic**: add, sub, addi, slt, sltu, slti, sltiu, and, or, xor, andi, ori, xori
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
opcodes.py    -- opcode and funct3/funct7 constants
decode.py     -- instruction decoder (binary -> structured fields)
execute.py    -- instruction executor (structured fields -> state changes)
csr.py        -- Control and Status Register file for M-mode
encoding.py   -- helpers to construct machine code for testing
simulator.py  -- top-level simulator struct and factory
```

## Where it fits

```
Logic Gates -> Arithmetic -> CPU -> [RISC-V Simulator] -> Assembler -> Lexer -> Parser -> Compiler -> VM
```

## Installation

```bash
uv add coding-adventures-riscv-simulator
```

## Usage

```python
from riscv_simulator import RiscVSimulator
from riscv_simulator.encoding import assemble, encode_addi, encode_add, encode_ecall

sim = RiscVSimulator()
program = assemble([
    encode_addi(1, 0, 1),    # x1 = 1
    encode_addi(2, 0, 2),    # x2 = 2
    encode_add(3, 1, 2),     # x3 = x1 + x2 = 3
    encode_ecall(),           # halt
])
traces = sim.run(program)
print(sim.cpu.registers.read(3))  # => 3
```

## Spec

See [07a-riscv-simulator.md](../../../specs/07a-riscv-simulator.md) for the full specification.
