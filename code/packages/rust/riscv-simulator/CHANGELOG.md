# Changelog

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
