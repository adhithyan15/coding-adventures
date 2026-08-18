# Changelog — CodingAdventures::RubyLexer (Perl)

All notable changes to this package are documented here.

## [Unreleased] — 2026-08-17

### Changed

- **Eliminated runtime grammar-file disk reads.** `_grammar()` previously
  opened `code/grammars/ruby/ruby.tokens` off disk (via `_grammars_dir()`
  climbing 5 directory levels from `__FILE__` with `File::Basename::dirname`
  and `File::Spec`) and parsed it with
  `CodingAdventures::GrammarTools::parse_token_grammar` on first use. A real
  CPAN install of this package would not ship `code/grammars/`, so that path
  would die with "cannot open ... No such file or directory" outside this
  monorepo checkout.
- `ruby.tokens` is now compiled once, at dev time, via
  `code/programs/perl/grammar-tools/grammar-tools.pl compile-tokens` into a
  checked-in generated module, `CodingAdventures::RubyLexer::_Grammar`
  (`lib/CodingAdventures/RubyLexer/_Grammar.pm`), which defines
  `token_grammar()`. `_grammar()` now `require`s that module and calls the
  qualified sub instead of touching disk.
- Removed the now-dead `_grammars_dir()` sub and the `use File::Basename
  qw(dirname);` / `use File::Spec;` imports (no longer used anywhere else
  in the module).
- Removed `File::Basename` and `File::Spec` from `Makefile.PL`'s
  `PREREQ_PM`.
- No behavioral change: `_build_rules()`, `tokenize()`, and the
  Perl-code-execution regex security check remain untouched — they operate
  on the same `TokenGrammar` object via the same accessor methods regardless
  of whether it came from disk-parsing or from the compiled module.

## [0.01] — 2026-03-29

### Added

- Initial implementation of `CodingAdventures::RubyLexer`.
- `tokenize($source)` — class method that tokenizes a Ruby string using
  the shared `ruby.tokens` grammar compiled to Perl `qr//` patterns.
- Grammar loaded from `code/grammars/ruby.tokens` and cached for the
  process lifetime using package-level variables.
- Path navigation uses `__FILE__` and `File::Basename::dirname` to locate
  the grammar file without hardcoded paths (climbs 5 levels from the module
  file to reach `code/`, then descends into `grammars/`).
- Full token set: NAME, NUMBER, STRING; keyword tokens (DEF, END, CLASS,
  MODULE, IF, ELSIF, ELSE, UNLESS, WHILE, UNTIL, FOR, DO, RETURN, BEGIN,
  RESCUE, ENSURE, REQUIRE, PUTS, YIELD, THEN, TRUE, FALSE, NIL, AND, OR,
  NOT); multi-char operator tokens (EQUALS_EQUALS, DOT_DOT, HASH_ROCKET,
  NOT_EQUALS, LESS_EQUALS, GREATER_EQUALS); single-char operator tokens
  (EQUALS, PLUS, MINUS, STAR, SLASH, LESS_THAN, GREATER_THAN); delimiter
  tokens (LPAREN, RPAREN, COMMA, COLON).
- Line and column tracking throughout tokenization.
- `die` with descriptive message on unexpected input characters.
- EOF sentinel always appended as last token.
- Comprehensive `Test2::V0` test suite covering all keywords (including
  Ruby-specific: elsif, unless, until, end, begin, rescue, ensure, yield,
  nil, module), identifiers, numbers, strings, operators (including
  Ruby-specific `..` and `=>`), punctuation, composite expressions,
  whitespace handling, position tracking, and error cases.
- `BUILD` script with transitive dependency installation in leaf-to-root
  order via cpanm.
- `BUILD_windows` stub (Perl testing not supported on Windows).
