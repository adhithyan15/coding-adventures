# Changelog — coding-adventures-vhdl-lexer

## [0.1.0] — 2026-03-29

### Added

- Initial implementation of `coding_adventures.vhdl_lexer`.
- Thin wrapper around the grammar-driven `GrammarLexer` from the `lexer` package.
- Loads `code/grammars/vhdl.tokens` at runtime (cached after first call) using
  `grammar_tools.parse_token_grammar`.
- VHDL is case-insensitive: `vhdl.tokens` sets `case_sensitive: false`, so
  the lexer lowercases all input before matching. All token values are lowercase.
- Public API: `tokenize(source)` and `get_grammar()`.
- `VERSION = "0.1.0"`.
- Full test suite in `tests/test_vhdl_lexer.lua` covering:
  - Module surface (loads, VERSION, tokenize, get_grammar)
  - Empty and whitespace/comment-only inputs produce only EOF
  - Structure keywords: entity, architecture, is, of, begin, end, port,
    generic, component, package, use, library
  - Type/signal keywords: signal, variable, constant, type, subtype,
    in, out, inout, buffer
  - Control flow: if, elsif, else, then, case, when, others, for, while,
    loop, process, wait
  - Operator keywords: and, or, not, nand, nor, xor, xnor
  - Two-char operators: <=, :=, =, /=, <, >, >=, =>, **
  - Single-char operators: +, -, *, /, &
  - Number literals: plain integer, underscore-separated
  - Bit string literals: X"FF", B"1010", O"77"
  - String literals, character literals ('0', '1')
  - VHDL -- line comment stripping
  - Case insensitivity: ENTITY/Entity/entity all same
  - Composite expressions (entity header, port declaration, etc.)
  - Whitespace/tab/newline stripping
  - Position tracking (line, col)
  - EOF sentinel
- `coding-adventures-vhdl-lexer-0.1.0-1.rockspec` with dependencies on
  grammar-tools, lexer, directed-graph, state-machine.
- `required_capabilities.json` declaring `filesystem:read` capability.
