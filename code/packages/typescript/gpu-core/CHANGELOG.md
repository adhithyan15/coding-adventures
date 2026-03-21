# Changelog

All notable changes to the `@coding-adventures/gpu-core` package.

## [0.1.0] - 2026-03-19

### Added
- Initial TypeScript implementation, ported from the Python `gpu-core` package.
- `FPRegisterFile` -- configurable floating-point register file (1-256 registers, FP32/FP16/BF16).
- `LocalMemory` -- byte-addressable scratchpad memory with FP-aware load/store.
- `Opcode` enum with all 16 opcodes: FADD, FSUB, FMUL, FFMA, FNEG, FABS, LOAD, STORE, MOV, LIMM, BEQ, BLT, BNE, JMP, NOP, HALT.
- `Instruction` interface and helper constructors (`fadd`, `fmul`, `limm`, `halt`, etc.) for readable GPU programs.
- `GenericISA` -- vendor-neutral instruction set implementation.
- `GPUCore` -- pluggable processing element with fetch-execute loop.
- `GPUCoreTrace` -- structured execution traces for educational visibility.
- `InstructionSet` and `ProcessingElement` interfaces for vendor-agnostic design.
- Full test suite covering registers, memory, opcodes, ISA, core execution, and integration programs (SAXPY, dot product, loops, conditionals).
