# Changelog — CodingAdventures::XmlLexer (Perl)

All notable changes to this package are documented here.

## [Unreleased] — 2026-08-17

### Changed

- **Eliminated runtime grammar-file disk reads.** `_grammar()` previously
  opened `code/grammars/xml/xml.tokens` off disk (via `_grammars_dir()`
  climbing 5 directory levels from `__FILE__` with `File::Basename::dirname`
  and `File::Spec`) and parsed it with
  `CodingAdventures::GrammarTools::parse_token_grammar` on first use. A real
  CPAN install of this package would not ship `code/grammars/`, so that path
  would die with "cannot open ... No such file or directory" outside this
  monorepo checkout.
- `xml.tokens` is now compiled once, at dev time, via
  `code/programs/perl/grammar-tools/grammar-tools.pl compile-tokens` into a
  checked-in generated module, `CodingAdventures::XmlLexer::_Grammar`
  (`lib/CodingAdventures/XmlLexer/_Grammar.pm`), which defines
  `token_grammar()`. `_grammar()` now `require`s that module and calls the
  qualified sub instead of touching disk.
- Removed the now-dead `_grammars_dir()` sub and the `use File::Basename
  qw(dirname);` / `use File::Spec;` imports (no longer used anywhere else
  in the module).
- Removed `File::Basename` and `File::Spec` from `Makefile.PL`'s
  `PREREQ_PM`.
- No behavioral change: rule compilation, the group-stack protocol,
  `tokenize()`, and the Perl-code-execution regex security check remain
  untouched.

## [0.02] — 2026-08-07

### Fixed
- **`xml.tokens` is now lookaround-free.** `COMMENT_TEXT`/`CDATA_TEXT`/
  `PI_TEXT` no longer use negative-lookahead regex; see
  `coding-adventures-xml-parser`'s (Rust) CHANGELOG for the full rationale.
  One behavior change: a comment/CDATA body containing the delimiter's
  ambiguous character (e.g. a lone `-` inside `<!-- a-b -->`) now surfaces
  as several adjacent `COMMENT_TEXT`/`CDATA_TEXT` tokens instead of one;
  concatenate them to get the original text. `CHAR_REF` is now produced by
  two aliased rules (`CHAR_REF_HEX`/`CHAR_REF_DEC`) instead of one rule
  with a top-level `A|B` pattern — no observable change to `CHAR_REF`
  tokens themselves.
- **Fixed a latent PI-body mis-tokenization bug**: `<?t a?b?>` had the `b`
  after the bare `?` wrongly re-tokenized as a second `PI_TARGET` instead
  of `PI_TEXT`, because the old single `pi` group offered `PI_TARGET`'s
  pattern for the whole PI body, not just the first token. The on-token
  callback now swaps from the `pi` group to a new `pi_body` group the
  instant `PI_TARGET` matches, so its pattern is never re-offered. Covered
  by a new regression test in `t/01-basic.t`.

## [0.01] — 2026-03-29

### Added

- Initial implementation of `CodingAdventures::XmlLexer`.
- `tokenize($source)` — tokenizes an XML string using rules compiled from
  the shared `xml.tokens` grammar file.
- Grammar is read from `code/grammars/xml.tokens` once and cached in
  package-level variables (`$_grammar`, `$_default_rules`, `$_group_rules`,
  `$_skip_rules`).
- Path navigation uses `File::Basename::dirname` and `File::Spec::rel2abs`
  relative to `__FILE__`, climbing 5 directory levels to the repo root.
- Pattern-group stack (`@_group_stack`) implements XML's context-sensitive
  lexical rules: switches between `default`, `tag`, `comment`, `cdata`, and
  `pi` groups based on tokens emitted.
- Skip patterns (whitespace) are only applied in `default` and `tag` groups;
  `comment`, `cdata`, and `pi` groups consume whitespace as part of their
  content tokens.
- Alias resolution: `ATTR_VALUE_DQ` and `ATTR_VALUE_SQ` emit as `ATTR_VALUE`.
- Line and column tracking for all tokens.
- `die` with a descriptive "LexerError" message (including active group name)
  on unexpected input.
- `t/00-load.t` — smoke test.
- `t/01-basic.t` — comprehensive test suite covering all XML token types,
  group switching, attributes, text content, entity refs, char refs, comments,
  CDATA, processing instructions, composite document, position tracking.
- `BUILD` and `BUILD_windows` scripts.
