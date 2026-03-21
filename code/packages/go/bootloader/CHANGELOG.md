# Changelog

## 0.1.0 — 2026-03-21

### Added
- `Bootloader` type with `Generate()` and `GenerateWithComments()` for RISC-V machine code generation
- `BootloaderConfig` with configurable entry address, kernel location, stack base
- `DefaultBootloaderConfig()` with conventional memory layout addresses
- `DiskImage` type simulating persistent storage with `LoadKernel()`, `LoadUserProgram()`, `ReadWord()`, `Data()`
- `AnnotatedInstruction` type pairing machine code with assembly and comments
- Boot protocol magic validation (0xB007CAFE) with halt on mismatch
- Word-by-word kernel copy loop from memory-mapped disk to RAM
- Stack pointer setup and unconditional jump to kernel entry
- `InstructionCount()` and `EstimateCycles()` utility methods
- Comprehensive test suite: code generation, execution on simulated CPU, disk image operations
- 97%+ test coverage
