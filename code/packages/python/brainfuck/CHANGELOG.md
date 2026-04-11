# Changelog

## 0.3.0 — 2026-04-10

### Added

- `lexer.py`: `Lexer` class and `tokenize(source)` function. Grammar-driven tokenizer using `brainfuck.tokens`. Each token is a dataclass with `type`, `value`, `line`, and `column` fields. Comment characters are silently skipped.
- `parser.py`: `Parser` class and `parse(source)` function. Grammar-driven parser using `brainfuck.grammar`. Returns an AST rooted at a `program` node with `instruction`, `loop`, and `command` child nodes. Raises `ParseError` with precise source-location details for unmatched brackets.
- Grammar files: `brainfuck.tokens` and `brainfuck.grammar` bundled in the package (shared across all language implementations).
- New dependencies: `coding-adventures-grammar-tools`, `coding-adventures-lexer`, and `coding-adventures-parser`.
- Extensive tests for `tokenize` (command tokens, comment skipping, multi-line source, position tracking) and `parse` (simple programs, nested loops, unmatched bracket errors with line/column).

## 0.1.0 — 2026-03-20

### Added

- Initial release of the Brainfuck interpreter
- 9 opcodes: RIGHT, LEFT, INC, DEC, OUTPUT, INPUT, LOOP_START, LOOP_END, HALT
- Translator: Brainfuck source → CodeObject with bracket matching
- Opcode handlers registered with GenericVM via `register_opcode()`
- Factory function `create_brainfuck_vm()` with tape (30,000 cells), data pointer, input buffer
- Convenience function `execute_brainfuck(source, input_data)` for one-call execution
- `BrainfuckResult` dataclass with output, tape, dp, traces, steps
- Cell wrapping (255+1=0, 0-1=255)
- EOF-on-input returns 0
- Full test suite: translator tests, handler unit tests, end-to-end programs including Hello World
