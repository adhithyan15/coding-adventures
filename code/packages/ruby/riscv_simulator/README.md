# RISC-V Simulator

Implements the full RISC-V RV32I base integer instruction set with M-mode
privileged extensions. Part of the **coding-adventures** computing stack.

## Supported Instructions

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
opcodes.rb   -- opcode and funct3/funct7 constants
decode.rb    -- instruction decoder for all six formats (R/I/S/B/U/J)
execute.rb   -- instruction executor for all operations
csr.rb       -- Control and Status Register file for M-mode
encoding.rb  -- helpers to construct machine code for testing
simulator.rb -- top-level simulator struct and factory
```

## Usage

```ruby
require "coding_adventures_riscv_simulator"

sim = CodingAdventures::RiscvSimulator::RiscVSimulator.new
program = CodingAdventures::RiscvSimulator.assemble([
  CodingAdventures::RiscvSimulator.encode_addi(1, 0, 1),
  CodingAdventures::RiscvSimulator.encode_addi(2, 0, 2),
  CodingAdventures::RiscvSimulator.encode_add(3, 1, 2),
  CodingAdventures::RiscvSimulator.encode_ecall
])
traces = sim.run(program)
puts sim.cpu.registers.read(3) # => 3
```
