# Changelog

All notable changes to the riscv-simulator package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-03-19

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
- Modular architecture split into separate files:
  - opcodes.py: opcode and funct3/funct7 constants
  - decode.py: expanded decoder for all six instruction formats (R/I/S/B/U/J)
  - execute.py: all instruction implementations
  - csr.py: CSR register file with read/write/set/clear operations
  - encoding.py: encoding helpers for all instructions
- 63 tests covering all instructions with 98% code coverage

## [0.1.0] - Unreleased

### Added
- Initial package scaffolding with pyproject.toml, src layout, and test structure
