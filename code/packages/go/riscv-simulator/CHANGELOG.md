# Changelog

## [0.4.0] - Unreleased

### Added
- Added opt-in host byte I/O syscalls for compiler end-to-end tests.

## [0.3.0] - 2026-04-02

### Changed
- Wrapped all public functions with the Operations system (`StartNew[T]`) for
  uniform observability, tracing, and error propagation across the package.
  Affected files: `simulator.go`, `encoding.go`, `core_adapter.go`.

## [0.2.0] - Unreleased

### Added
- Full RV32I base integer instruction set (~30 new instructions):
  - I-type arithmetic: slti, sltiu, xori, ori, andi, slli, srli, srai
  - R-type arithmetic: sll, slt, sltu, xor, srl, sra, or, and
  - Load instructions: lb, lh, lw, lbu, lhu (sign/zero extend)
  - Store instructions: sb, sh, sw
  - Branch instructions: beq, bne, blt, bge, bltu, bgeu
  - Jump instructions: jal (J-type), jalr (I-type)
  - Upper immediate: lui, auipc
- M-mode privileged extensions for OS support:
  - CSR register file with mstatus, mtvec, mepc, mcause, mscratch
  - CSR access instructions: csrrw, csrrs, csrrc
  - Trap return: mret (restores PC from mepc, re-enables interrupts)
  - ecall now raises a proper software trap when mtvec is configured
- Encoding helpers for all instruction formats (R, I, S, B, U, J, CSR)
- Comprehensive test suite (63 tests, 97% coverage) including:
  - Individual instruction tests for all arithmetic, load/store, branch, jump operations
  - Encode-decode round-trip verification for all instructions
  - CSR file unit tests
  - ecall trap handling with handler setup and mret return
  - Integration tests: Fibonacci, memcpy, stack push/pop, function call/return

### Changed
- Refactored into multi-file architecture: opcodes.go, decode.go, execute.go, csr.go, encoding.go
- `RiscVExecutor` now holds a `CSR` field for privileged operations
- `RiscVSimulator` now includes a `CSRFile` for M-mode registers
- ecall behavior: raises trap when mtvec is set, halts as fallback when mtvec=0
- Decoder now uses switch-based dispatch on opcode for all instruction formats
- Encoding helpers moved to dedicated encoding.go file

## [0.1.0] - Unreleased

### Added
- `RiscVDecoder` implementing RISC-V RV32I opcodes mapping.
- `RiscVExecutor` mapping decoded `cpu.DecodeResult` to arithmetic manipulations bridging the `cpu-simulator` generic execution loop.
- Enforcement of `x0` constant hardwiring to zero.
- Complete documentation adhering to literate programming standards.
- Helper testing encoders explicitly producing Little Endian binary sequences.
