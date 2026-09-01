# Changelog

## Unreleased

- Added the normative `Rv32ISimulator` machine boundary with exact 64 KiB
  state, five M-mode CSRs, checked origin-aware loading and restore/direct
  access, typed fail-closed faults, complete transition traces, atomic steps,
  and transactional bounded runs while preserving every legacy consumer API.
- Added seven lifecycle/fault suites and a reproducible 256-case Python
  full-state differential corpus spanning every RV32I/CSR/MRET decode family.
- Corrected the checked boundary to reject malformed funct fields, selected
  RV32M encodings, non-ECALL privileged words, and unsupported CSRs instead of
  silently advancing them as unknown operations.
- Formatted the existing Rust package and established 91.46% line coverage
  (1,125/1,230) with strict Rustfmt, Clippy, and rustdoc checks green.
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
