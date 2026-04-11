# Changelog

## 0.3.0 -- 2026-04-10

### Added

- `lexer.ts`: `Lexer` class and `tokenize(source)` function. Grammar-driven tokenizer using `brainfuck.tokens`. Each token carries `type`, `value`, `line`, and `column` fields. Comment characters are silently skipped.
- `parser.ts`: `Parser` class and `parse(source)` function. Grammar-driven parser using `brainfuck.grammar`. Returns an AST rooted at a `program` node with `instruction`, `loop`, and `command` child nodes. Emits precise source-location errors for unmatched brackets.
- Grammar files: `brainfuck.tokens` and `brainfuck.grammar` bundled as package assets (shared across all language implementations).
- New dependencies: `@coding-adventures/grammar-tools`, `@coding-adventures/lexer`, and `@coding-adventures/parser`.
- Extensive tests for `tokenize` (command tokens, comment skipping, multi-line source, position tracking) and `parse` (simple programs, nested loops, unmatched bracket errors).

## 0.1.0 -- 2026-03-20

### Added

- Initial release of the Brainfuck interpreter (TypeScript)
- 9 opcodes: RIGHT, LEFT, INC, DEC, OUTPUT, INPUT, LOOP_START, LOOP_END, HALT
- Translator: Brainfuck source -> CodeObject with bracket matching
- Opcode handlers registered with GenericVM via `registerOpcode()`
- Factory function `createBrainfuckVm()` with tape (30,000 cells), data pointer, input buffer
- Convenience function `executeBrainfuck(source, inputData)` for one-call execution
- `BrainfuckResult` interface with output, tape, dp, traces, steps
- Cell wrapping (255+1=0, 0-1=255)
- EOF-on-input returns 0
- Full test suite: translator tests, handler unit tests, end-to-end programs including Hello World
