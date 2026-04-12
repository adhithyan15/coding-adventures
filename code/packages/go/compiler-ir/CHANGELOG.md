# Changelog

All notable changes to the `compiler-ir` package will be documented in
this file.

## [0.1.0] — 2026-04-12

### Added

- IR opcode enumeration with 25 instructions across 7 categories
  (constants, memory, arithmetic, comparison, control flow, system, meta)
- Operand types: `IrRegister`, `IrImmediate`, `IrLabel` with `IrOperand`
  interface
- `IrInstruction` with unique monotonic ID for source mapping
- `IrDataDecl` for data segment declarations (.bss/.data)
- `IrProgram` container with instructions, data, entry label, and version
- `IDGenerator` for producing unique instruction IDs
- `Print()` function converting IrProgram to canonical text format
- `Parse()` function converting text back to IrProgram
- `ParseOp()` for text-to-opcode conversion
- IR version directive (`.version N`) — v1 is the Brainfuck subset
- Full test suite with roundtrip verification (parse(print(p)) == p)
