# Changelog

All notable changes to the Brainfuck interpreter (Go) package.

## [0.2.0] - 2026-03-31

### Changed

- Wrapped all public functions and methods (`NewBrainfuckVM`, `Execute`, `Step`, `CreateBrainfuckVM`, `ExecuteBrainfuck`, `Translate`) with the Operations system for automatic timing, structured logging, and panic recovery.
- Added private `step` helper to avoid nested Operation instrumentation inside `Execute`.

## [0.1.0] - 2026-03-20

### Added

- `opcodes.go`: Brainfuck opcode constants (`OpRight`, `OpLeft`, `OpInc`, `OpDec`, `OpOutput`, `OpInput`, `OpLoopStart`, `OpLoopEnd`, `OpHalt`) using the `vm.OpCode` type from the virtual-machine package. Character-to-opcode mapping via `CharToOp`.
- `translator.go`: `Translate()` function that converts Brainfuck source code into a `vm.CodeObject`. Handles bracket matching with a stack, panics on mismatched brackets.
- `handlers.go`: `BrainfuckVM` struct with tape (30,000 byte cells), data pointer, program counter, input buffer, and output. `Execute()` and `Step()` methods implementing all 9 opcodes with cell wrapping (0-255) and boundary checking.
- `vm.go`: `BrainfuckResult` struct, `CreateBrainfuckVM()` factory, and `ExecuteBrainfuck()` convenience function for one-call execution.
- `translator_test.go`: 16 tests covering basic translation, bracket matching, and bracket error cases.
- `handlers_test.go`: 24 tests covering pointer movement, cell modification, I/O, control flow, and VM state initialization.
- `vm_test.go`: 17 end-to-end tests including Hello World, input/output, nested loops, cell wrapping, comments, and result field verification.
- Full literate programming documentation throughout all source files.
