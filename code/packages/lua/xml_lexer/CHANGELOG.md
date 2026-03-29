# Changelog — coding-adventures-xml-lexer (Lua)

All notable changes to this package are documented here.

## [0.1.0] — 2026-03-29

### Added

- Initial implementation of `coding_adventures.xml_lexer`.
- `tokenize(source)` — tokenizes an XML string using the shared `xml.tokens`
  grammar and the grammar-driven `GrammarLexer` from `coding-adventures-lexer`.
- `get_grammar()` — returns the cached `TokenGrammar` for direct use.
- Group-switching `on_token` callback that drives context-sensitive lexing:
  pushes/pops pattern groups (`tag`, `comment`, `cdata`, `pi`) as structural
  tokens are emitted.
- Grammar is read from `code/grammars/xml.tokens` once and cached.
- Path navigation uses `debug.getinfo` to locate the grammar file relative to
  the installed module, avoiding hardcoded absolute paths.
- Comprehensive busted test suite covering all XML token types: opening/closing
  tags, self-closing tags, attributes (double- and single-quoted), text content,
  entity references, character references, comments, CDATA sections, processing
  instructions, and full-document round-trip.
- `required_capabilities.json` declaring `filesystem:read`.
- `BUILD` and `BUILD_windows` scripts with transitive dependency installation
  in leaf-to-root order.
