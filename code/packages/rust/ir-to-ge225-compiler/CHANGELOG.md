# Changelog

## Unreleased

- add the first Rust `ir-to-ge225-compiler` backend
- port the three-pass GE-225 assembler from Python to Rust
- implement `validate_for_ge225` checking opcode support, 20-bit constant range,
  SYSCALL number, and AND_IMM immediate constraints
- implement `compile_to_ge225` with Pass 0 (register/constant collection),
  Pass 1 (label address assignment), and Pass 2 (word emission)
- add `GE225CodeGenerator` adapter implementing the `CodeGenerator<IrProgram, CompileResult>` protocol
- write 30+ unit tests covering every opcode emitter and validation rule
