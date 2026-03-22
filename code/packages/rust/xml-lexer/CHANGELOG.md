# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- Initial release of the XML lexer crate.
- `create_xml_lexer()` factory function returning a `GrammarLexer` with the XML on-token callback registered.
- `tokenize_xml()` convenience function returning `Vec<Token>` directly.
- `xml_on_token()` callback function that drives context-sensitive lexing via pattern group transitions.
- Loads the `xml.tokens` grammar file at runtime from the shared `grammars/` directory.
- Supports 5 pattern groups: default (text/entities), tag (names/attributes), comment, cdata, and pi.
- Callback pushes/pops groups on tag/comment/CDATA/PI boundaries and toggles skip for whitespace-significant groups.
- 28 unit tests covering basic tags, namespaces, self-closing tags, attributes (single/double quoted), comments (with whitespace preservation and dashes), CDATA sections (with angle brackets and single brackets), processing instructions, entity references (named, decimal, hex), nested elements, mixed content, full documents, edge cases (empty input, text only, whitespace skipping, EOF), and deeply nested structures.
