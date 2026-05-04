# Changelog

## Unreleased

- add the first Rust `ir-to-intel-8008-compiler` backend
- port the one-pass Intel 8008 assembly generator from Python to Rust
- implement `IrValidator` with 5 hardware-constraint validation rules
  (no LOAD_WORD/STORE_WORD, register count ≤ 6, u8 immediates,
   SYSCALL whitelist, static data ≤ 8 191 bytes)
- implement `IrToIntel8008Compiler` emitting correct 8008 assembly text
- handle the dangerous MOV A,C / MOV A,H / MOV A,M opcode conflicts
  via the safe group-10 ALU path (MVI A, 0; ADD reg)
- implement SYSCALL expansion for ADC, SBB, rotations (RLC/RRC/RAL/RAR),
  carry materialisation, parity materialisation, IN/OUT port I/O
- add `Intel8008CodeGenerator` adapter implementing the
  `CodeGenerator<IrProgram, String>` LANG20 protocol
- write 35+ unit tests covering every opcode emitter and validation rule
