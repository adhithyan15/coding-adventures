# Changelog — CodingAdventures::VerilogLexer

## [0.01] — 2026-03-29

### Added

- Initial implementation of `CodingAdventures::VerilogLexer`.
- Thin wrapper around `CodingAdventures::GrammarTools::parse_token_grammar`.
- Reads `code/grammars/verilog.tokens` at process startup (cached).
- Compiles token definitions to `qr/\G.../` Perl patterns for efficient
  single-pass tokenization using Perl's `\G` anchor + `pos()` mechanism.
- `tokenize($source)` returns arrayref of token hashrefs with keys:
  `type`, `value`, `line`, `col`. Last element always `type => 'EOF'`.
- `VERSION = '0.01'`.
- Test suite in `t/00-load.t` and `t/01-basic.t` covering:
  - Module loads and has VERSION
  - Empty string and whitespace-only produce only EOF
  - Line comment (`//`) and block comment (`/* */`) consumed silently
  - Module structure keywords: module, endmodule, input, output, inout,
    reg, wire, parameter, localparam
  - Control flow: always, initial, begin, end, if, else, case, casez,
    casex, endcase, for
  - Gate primitives: and, or, not, nand, nor, xor, xnor, buf
  - Number literals: plain decimal, sized hex/binary/octal, x/z states
  - Operators: =, <=, ==, !=, &, |, ^, ~, <<, >>, +, -, *, /, >=, <
  - Special tokens: $system_id, `directive, #delay, @event
  - String literals
  - Composite expressions (module declaration, always block, etc.)
  - Whitespace/tab/newline stripping
  - Position tracking (line, col)
  - EOF sentinel
  - Error handling (unexpected characters)
- `cpanfile` declaring dependencies on GrammarTools and Lexer.
- `Makefile.PL` with full metadata.
