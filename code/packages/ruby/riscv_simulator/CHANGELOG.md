# Changelog

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
  - opcodes.rb, decode.rb, execute.rb, csr.rb, encoding.rb
- 63 tests covering all instructions (ported from Go reference)

## [0.1.0] - 2026-03-18

### Added
- RiscVDecoder: decodes addi (I-type), add/sub (R-type), ecall
- RiscVExecutor: executes decoded RISC-V instructions with x0 hardwired to zero
- Assembler helpers: encode_addi, encode_add, encode_sub, encode_ecall, assemble
- RiscVSimulator: high-level wrapper combining CPU + RISC-V decoder/executor
