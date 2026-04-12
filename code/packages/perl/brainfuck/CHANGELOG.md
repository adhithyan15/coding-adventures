# Changelog — CodingAdventures::Brainfuck (Perl)

## [0.03] — 2026-04-10

### Added
- `CodingAdventures::Brainfuck::Lexer`: Grammar-driven tokenizer using `brainfuck.tokens`. `tokenize($source)` returns an array ref of token hashrefs with `type`, `value`, `line`, and `column` keys. Comment characters are silently skipped.
- `CodingAdventures::Brainfuck::Parser`: Grammar-driven parser using `brainfuck.grammar`. `parse($source)` returns an AST as a hashref rooted at `{type => "program"}` with `instruction`, `loop`, and `command` child nodes. Returns `(undef, $message)` with precise source-location details for unmatched brackets.
- Grammar files: `brainfuck.tokens` and `brainfuck.grammar` installed with the distribution (shared across all language implementations).
- New dependencies: `CodingAdventures::GrammarTools`, `CodingAdventures::Lexer`, and `CodingAdventures::Parser`.
- Extensive tests for `tokenize` (command tokens, comment skipping, multi-line source, position tracking) and `parse` (simple programs, nested loops, unmatched bracket errors with line/column).

## [0.01] — 2026-03-29

### Added
- `validate($program)` — bracket balance checker.
- `compile_to_opcodes($program)` — two-pass compiler: opcodes + stack-based jump target resolution.
- `run_opcodes($opcodes, $input)` — eval loop with correct cell wrapping (0→255, 255→0), EOF-as-zero, and bounds checking.
- `interpret($program, $input)` — high-level one-call interface.
- Opcode constants: `OP_RIGHT`, `OP_LEFT`, `OP_INC`, `OP_DEC`, `OP_OUTPUT`, `OP_INPUT`, `OP_LOOP_START`, `OP_LOOP_END`, `OP_HALT`.
- Test suite: validation, compilation, all 8 commands, cell wrapping, loops (skip/execute/copy), input/EOF, Hello World multiplication pattern.
