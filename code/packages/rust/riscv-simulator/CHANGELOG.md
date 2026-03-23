# Changelog

## [0.1.0] - 2026-03-19

### Added
- Full RV32I base integer instruction set (37 instructions)
- M-mode privileged extensions (csrrw, csrrs, csrrc, mret, ecall trap handling)
- CSR registers: mstatus, mtvec, mepc, mcause, mscratch
- Modular architecture: opcodes, decode, execute, csr, encoding, simulator
- 63+ tests covering all instructions
