# Changelog

All notable changes to the XML Lexer package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- Initial release of the XML lexer for TypeScript.
- `tokenizeXML()` function that tokenizes XML text using pattern groups and the on-token callback.
- `createXMLLexer()` function that returns a configured `GrammarLexer` instance.
- `xmlOnToken()` callback that drives context-sensitive group switching for XML tokenization.
- Loads `xml.tokens` grammar file defining 5 pattern groups: default, tag, comment, cdata, pi.
- Full support for XML elements: open tags, close tags, self-closing tags, namespaced tags.
- Full support for attributes: double-quoted and single-quoted values.
- Full support for comments (`<!-- -->`), CDATA sections (`<![CDATA[ ]]>`), and processing instructions (`<? ?>`).
- Entity references (`&amp;`) and character references (`&#65;`, `&#x41;`).
- Whitespace preservation inside comments, CDATA, and PIs (skip patterns disabled via callback).
- Comprehensive test suite covering basic tags, attributes, comments, CDATA, PIs, entity references, nested/mixed content, edge cases, and position tracking.
