# Changelog

## Unreleased

- Kept ECALL halt-token classification warning-free under the current stable
  Clippy without changing trap behavior.
- Added the standard RV32M `mul`, `mulhu`, `div`, `divu`, `rem`, and `remu`
  instructions to both simulator execution paths and the test encoder,
  including RISC-V-defined division-by-zero and signed-overflow results.
- Added bounded `run_loaded_with_limit` execution with an observable
  `ExecutionResult`, allowing compiler backends to distinguish a halted guest
  from one that exhausted its instruction budget.

## [0.1.0] - 2026-03-19

### Added
- Full RV32I base integer instruction set (37 instructions)
- M-mode privileged extensions (csrrw, csrrs, csrrc, mret, ecall trap handling)
- CSR registers: mstatus, mtvec, mepc, mcause, mscratch
- Modular architecture: opcodes, decode, execute, csr, encoding, simulator
- 63+ tests covering all instructions
