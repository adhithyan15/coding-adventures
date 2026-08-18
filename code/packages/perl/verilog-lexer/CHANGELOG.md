# Changelog — CodingAdventures::VerilogLexer

## [0.02] — 2026-08-17

### Changed

- Eliminated runtime disk reads of `.tokens` grammar files. Each of the 3
  supported versions (`1995`, `2001`, `2005`) is now compiled once, at dev
  time, into a native Perl module checked into git under
  `lib/CodingAdventures/VerilogLexer/_Grammar_*.pm`, via:
  `grammar-tools.pl compile-tokens <file.tokens> -o <output.pm> -p <Package::Name>`.
  A real CPAN install of this package does not ship `code/grammars/`, so the
  old `open()`-based `_grammar()` would have died with "No such file or
  directory" outside this monorepo checkout.
- `_grammar($version)` now dispatches through `%GRAMMAR_MODULE` (keyed by
  resolved version string) and calls the matching module's
  `token_grammar()` sub instead of parsing a `.tokens` file off disk.
  Note: the unversioned `verilog.tokens` file was never read by this module
  (only `verilog1995.tokens`, `verilog2001.tokens`, `verilog2005.tokens`
  via `_resolve_version()`'s default), so it was not compiled.
- Removed `_grammars_dir()` (dead code — no more path navigation needed)
  and the now-unused `File::Basename` / `File::Spec` imports. `Makefile.PL`'s
  `PREREQ_PM` no longer lists them.
- `$VERSION` bumped from `0.01` to `0.02`.

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
