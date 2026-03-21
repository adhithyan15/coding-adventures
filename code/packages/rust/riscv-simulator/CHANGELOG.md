# Changelog

<<<<<<< HEAD
## [0.1.0] - 2026-03-19

### Added
- Full RV32I base integer instruction set (37 instructions)
  - I-type arithmetic: addi, slti, sltiu, xori, ori, andi, slli, srli, srai
  - R-type arithmetic: add, sub, sll, slt, sltu, xor, srl, sra, or, and
  - Load instructions: lb, lh, lw, lbu, lhu (with sign/zero extension)
  - Store instructions: sb, sh, sw
  - Branch instructions: beq, bne, blt, bge, bltu, bgeu
  - Jump instructions: jal, jalr
  - Upper immediate: lui, auipc
- M-mode privileged extensions
  - CSR access instructions: csrrw, csrrs, csrrc
  - Trap return: mret
  - CSR registers: mstatus, mtvec, mepc, mcause, mscratch
  - Trap handling: ecall triggers proper trap when mtvec is set
- Modular architecture: opcodes, decode, execute, csr, encoding, simulator
- 63 tests covering all instructions (ported from Go reference)
=======
All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- `RiscVDecoder` -- decodes I-type (addi), R-type (add, sub), and system (ecall) instructions
- `RiscVExecutor` -- executes decoded instructions against registers and memory
- `RiscVSimulator` -- full simulation environment wrapping the generic CPU
- Encoding helpers: `encode_addi`, `encode_add`, `encode_sub`, `encode_ecall`, `assemble`
- x0 hardwired-to-zero enforcement in all write paths
- Sign extension for 12-bit immediate values
- Comprehensive test suite covering normal operation, edge cases, and unknown instructions
>>>>>>> origin/main
