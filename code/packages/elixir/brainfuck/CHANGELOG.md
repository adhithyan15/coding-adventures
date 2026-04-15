# Changelog

## 0.3.0 — 2026-04-10

### Added
- `CodingAdventures.Brainfuck.Lexer`: Grammar-driven tokenizer using `brainfuck.tokens`. `tokenize/1` returns a list of token maps with `:type`, `:value`, `:line`, and `:column` keys. Comment characters are silently skipped.
- `CodingAdventures.Brainfuck.Parser`: Grammar-driven parser using `brainfuck.grammar`. `parse/1` returns an AST rooted at a `%{type: :program}` node with `:instruction`, `:loop`, and `:command` child nodes. Returns `{:error, message}` with precise source-location details for unmatched brackets.
- Grammar files: `brainfuck.tokens` and `brainfuck.grammar` bundled as priv assets (shared across all language implementations).
- New dependencies: `:coding_adventures_grammar_tools`, `:coding_adventures_lexer`, and `:coding_adventures_parser`.
- Extensive tests for `tokenize/1` (command tokens, comment skipping, multi-line source, position tracking) and `parse/1` (simple programs, nested loops, unmatched bracket errors with line/column).

## 0.1.0 — 2026-03-20

### Added
- Initial release
- Brainfuck interpreter built on the pluggable GenericVM
- Translator: source -> bytecode with bracket matching
- 9 opcode handlers using GenericVM's extra state
- Factory function and convenience executor
- BrainfuckResult struct with output, tape, dp, traces, steps
- Full Hello World support
- Input handling with EOF producing 0
- Cell wrapping (255+1=0, 0-1=255)
- Pointer bounds checking with clear error messages
- 78 tests at 96.84% coverage
