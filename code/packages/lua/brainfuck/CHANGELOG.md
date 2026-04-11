# Changelog — coding-adventures-brainfuck (Lua)

## [0.3.0] — 2026-04-10

### Added
- `lexer.lua`: `Lexer` table and `tokenize(source)` function. Grammar-driven tokenizer using `brainfuck.tokens`. Each token is a table with `type`, `value`, `line`, and `column` fields. Comment characters are silently skipped.
- `parser.lua`: `Parser` table and `parse(source)` function. Grammar-driven parser using `brainfuck.grammar`. Returns an AST rooted at a `program` node table with `instruction`, `loop`, and `command` child nodes. Returns `nil, message` with precise source-location details for unmatched brackets.
- Grammar files: `brainfuck.tokens` and `brainfuck.grammar` bundled in the package (shared across all language implementations).
- New dependencies: `coding-adventures-grammar-tools`, `coding-adventures-lexer`, and `coding-adventures-parser` Lua rocks.
- Extensive tests for `tokenize` (command tokens, comment skipping, multi-line source, position tracking) and `parse` (simple programs, nested loops, unmatched bracket errors with line/column).

## [0.1.0] — 2026-03-29

### Added
- `bf.validate(program)` — bracket balance checker.
- `bf.compile_to_opcodes(program)` — translates source to opcode list with pre-computed jump targets using stack-based bracket matching.
- `bf.run_opcodes(opcodes, input_str)` — eval loop executing all 8 Brainfuck commands with correct cell wrapping (0→255, 255→0) and EOF-as-zero convention.
- `bf.interpret(program, input_str)` — high-level one-call interface.
- Opcode constants: `OP_RIGHT`, `OP_LEFT`, `OP_INC`, `OP_DEC`, `OP_OUTPUT`, `OP_INPUT`, `OP_LOOP_START`, `OP_LOOP_END`, `OP_HALT`.
- Test suite: validation, compilation, all 8 commands, cell wrapping, loops, input/EOF, Hello World 'H' multiplication pattern.
