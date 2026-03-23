# Changelog

All notable changes to the `coding_adventures_gpu_core` gem will be documented
in this file.

## [0.1.0] - 2026-03-19

### Added

- `FPRegisterFile` -- configurable floating-point register file (1-256 registers, FP32/FP16/BF16)
- `LocalMemory` -- byte-addressable scratchpad memory with IEEE 754 float load/store
- `Instruction` -- immutable instruction representation using `Data.define`
- 16 opcodes: FADD, FSUB, FMUL, FFMA, FNEG, FABS, LOAD, STORE, MOV, LIMM, BEQ, BLT, BNE, JMP, NOP, HALT
- Helper constructors for all opcodes (e.g., `GpuCore.fadd(2, 0, 1)`)
- `ExecuteResult` -- immutable result type from instruction execution
- `GenericISA` -- vendor-neutral educational instruction set implementation
- `GPUCore` -- the main fetch-execute loop with pluggable ISA
- `GPUCoreTrace` -- structured execution trace records with `#format` for display
- Full test suite: registers, memory, opcodes, ISA, core, and integration programs
- Knuth-style literate programming comments throughout all source files
- Ruby port of the Python `gpu-core` package
