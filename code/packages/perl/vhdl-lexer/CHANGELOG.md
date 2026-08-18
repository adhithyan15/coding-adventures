# Changelog — CodingAdventures::VhdlLexer

## [0.02] — 2026-08-17

### Changed

- Eliminated runtime disk reads of `.tokens` grammar files. Each of the 5
  supported versions (`1987`, `1993`, `2002`, `2008`, `2019`) is now
  compiled once, at dev time, into a native Perl module checked into git
  under `lib/CodingAdventures/VhdlLexer/_Grammar_*.pm`, via:
  `grammar-tools.pl compile-tokens <file.tokens> -o <output.pm> -p <Package::Name>`.
  A real CPAN install of this package does not ship `code/grammars/`, so the
  old `open()`-based `_grammar()` would have died with "No such file or
  directory" outside this monorepo checkout.
- `_grammar($version)` now dispatches through `%GRAMMAR_MODULE` (keyed by
  resolved version string) and calls the matching module's
  `token_grammar()` sub instead of parsing a `.tokens` file off disk.
  Note: the unversioned `vhdl.tokens` file was never read by this module
  (only `vhdl1987.tokens` ... `vhdl2019.tokens` via `_resolve_version()`'s
  default), so it was not compiled.
- Removed `_grammars_dir()` (dead code — no more path navigation needed)
  and the now-unused `File::Basename` / `File::Spec` imports. `Makefile.PL`'s
  `PREREQ_PM` no longer lists them.
- `$VERSION` bumped from `0.01` to `0.02`.

## [0.01] — 2026-03-29

### Added

- Initial implementation of `CodingAdventures::VhdlLexer`.
- Thin wrapper around `CodingAdventures::GrammarTools::parse_token_grammar`.
- Reads `code/grammars/vhdl.tokens` at process startup (cached).
- VHDL is case-insensitive: `vhdl.tokens` sets `case_sensitive: false`,
  so all input is lowercased before matching. All returned token values
  are lowercase.
- Compiles token definitions to `qr/\G.../` Perl patterns for efficient
  single-pass tokenization using Perl's `\G` anchor + `pos()` mechanism.
- `tokenize($source)` returns arrayref of token hashrefs with keys:
  `type`, `value`, `line`, `col`. Last element always `type => 'EOF'`.
- `VERSION = '0.01'`.
- Test suite in `t/00-load.t` and `t/01-basic.t` covering:
  - Module loads and has VERSION
  - Empty string and whitespace-only produce only EOF
  - VHDL `--` line comment consumed silently
  - Structure keywords: entity, architecture, is, of, begin, end, port,
    generic, component, package, use, library
  - Type/signal keywords: signal, variable, constant, type, subtype,
    in, out, inout, buffer
  - Control flow: if, elsif, else, then, case, when, others, for, while,
    loop, process, wait
  - Operator keywords: and, or, not, nand, nor, xor, xnor
  - Symbol operators: <=, :=, =, /=, =>, **, <, >, >=, +, -, *, /, &
  - Number literals: plain integer, underscore-separated
  - Bit string literals: X"FF", B"1010", O"77"
  - String and character literals ('0', '1')
  - Case insensitivity: ENTITY/Entity/entity all same, values lowercase
  - Composite expressions (entity/architecture headers, port declaration,
    if/elsif/else, case/when/others, process, constant declaration, etc.)
  - Whitespace/tab/newline stripping
  - Position tracking (line, col)
  - EOF sentinel
  - Error handling (unexpected characters)
- `cpanfile` declaring dependencies on GrammarTools and Lexer.
- `Makefile.PL` with full metadata.
