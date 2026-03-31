# Changelog — coding-adventures-verilog-lexer

## [0.1.0] — 2026-03-29

### Added

- Initial implementation of `coding_adventures.verilog_lexer`.
- Thin wrapper around the grammar-driven `GrammarLexer` from the `lexer` package.
- Loads `code/grammars/verilog.tokens` at runtime (cached after first call) using
  `grammar_tools.parse_token_grammar`.
- Public API: `tokenize(source)` and `get_grammar()`.
- `VERSION = "0.1.0"`.
- Full test suite in `tests/test_verilog_lexer.lua` covering:
  - Module surface (loads, VERSION, tokenize function, get_grammar function)
  - Empty and whitespace/comment-only inputs produce only EOF
  - Module structure keywords: module, endmodule, input, output, inout, reg,
    wire, parameter, localparam
  - Control flow keywords: always, initial, begin, end, if, else, case, casez,
    casex, endcase, for
  - Gate primitive keywords: and, or, not, nand, nor, xor, xnor, buf
  - Number literals: plain integer, sized hex/binary/octal/decimal, x/z states
  - Operators: =, <=, ==, !=, &, |, ^, ~, <<, >>, +, -, *, /, **, >=, <, >
  - Special tokens: $system_id, `directive, #delay, @event
  - String literals
  - Delimiter tokens
  - Composite expressions (module declaration, wire range, always block, etc.)
  - Whitespace/tab/newline stripping
  - Position tracking (line, col)
  - EOF sentinel
- `coding-adventures-verilog-lexer-0.1.0-1.rockspec` with dependencies on
  grammar-tools, lexer, directed-graph, state-machine.
- `required_capabilities.json` declaring `filesystem:read` capability.
