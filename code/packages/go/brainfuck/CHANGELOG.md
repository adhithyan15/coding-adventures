# Changelog

All notable changes to the Brainfuck interpreter (Go) package.

## [0.3.0] - 2026-04-10

### Added

- `lexer.go`: `Lexer` struct and `Tokenize(source string) []Token` function. Grammar-driven tokenizer using `brainfuck.tokens`. Each token carries `Type`, `Value`, `Line`, and `Column` fields for precise source location. Comment characters are silently skipped.
- `parser.go`: `Parser` struct and `Parse(source string) *ASTNode` function. Grammar-driven parser using `brainfuck.grammar`. Returns an AST rooted at a `program` node with `instruction`, `loop`, and `command` child nodes. Emits precise source-location errors for unmatched brackets.
- `_tokens_grammar.go`: Embedded `brainfuck.tokens` grammar file (shared across all language implementations).
- `_parser_grammar.go`: Embedded `brainfuck.grammar` grammar file (shared across all language implementations).
- New dependencies: `grammar-tools`, `lexer`, and `parser` packages from the coding-adventures stack.
- `lexer_test.go`: Extensive tests for tokenization — command tokens, comment skipping, multi-line source, empty input, and token position tracking.
- `parser_test.go`: Extensive tests for AST construction — simple programs, nested loops, loop skip/execute paths, unmatched bracket errors with line/column reporting.

## [0.2.1] - 2026-04-02

### Fixed

- Added `.PanicOnUnexpected()` to `Execute`, `Step`, and `Translate` operations so intentional panics (tape boundary violations, unmatched brackets) propagate correctly instead of being swallowed by the Operations panic-recovery wrapper.

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
