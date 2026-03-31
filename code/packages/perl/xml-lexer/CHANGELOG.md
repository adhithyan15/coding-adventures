# Changelog — CodingAdventures::XmlLexer (Perl)

All notable changes to this package are documented here.

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
