# Changelog

All notable changes to the `compiler-ir` package will be documented in
this file.

## [Unreleased] — 2026-10-21

### Added

- Add strict BF03 `ValidateCoreProfile`, `ParseCoreProfile`, `PrintCoreProfile`
  and `CoreIDSequence`, plus distinct inert `IrText` COMMENT operands.
- Enforce closed25operation roles, canonical ASCII/LF text, numeric/name/tag
  and allocation bounds, metadata/ordinary ID policy and atomic exhaustion.
- Consume all50neutral corpus cases with bounded duplicate/Unicode/number/tree
  admission, independent literal/digest anchors and adversarial mutations.
- Test nil/pointer/malformed COMMENT errors, owned results, legacy Nib comments,
  all operand roles, boundary budgets and generated pure outcome behavior.

### Compatibility

- Preserve existing broad Parse/Print, opcodes, mutable Add methods and legacy
  IDGenerator behavior. Strict rejection does not normalize legacy debug data.
- No source-map, optimizer, backend/runtime or whole-language qualification.

### Fixed

- Check fixture integers against native bounds before narrowing, and route
  sequence counts through the same checked conversion. Exercise both native
  boundaries and rejection of values outside a 32-bit integer domain.

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
