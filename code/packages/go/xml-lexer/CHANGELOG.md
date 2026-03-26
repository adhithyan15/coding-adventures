# Changelog

All notable changes to the xml-lexer package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- Initial implementation of the XML lexer wrapper around the grammar-driven lexer
- `XmlOnToken` callback function for context-sensitive pattern group switching:
  - Pushes "tag" group on `OPEN_TAG_START` or `CLOSE_TAG_START`
  - Pops group on `TAG_CLOSE` or `SELF_CLOSE`
  - Pushes "comment" group and disables skip on `COMMENT_START`
  - Pops and re-enables skip on `COMMENT_END`
  - Same push/pop + skip toggle pattern for CDATA and PI groups
- `CreateXmlLexer(source string)` factory function for creating reusable lexer instances
- `TokenizeXml(source string)` convenience function for one-shot tokenization
- Grammar file loading via `runtime.Caller(0)` for location-independent operation
- Support for all XML token types across 5 pattern groups:
  - Default: TEXT, ENTITY_REF, CHAR_REF, tag/comment/cdata/pi openers
  - Tag: TAG_NAME, ATTR_EQUALS, ATTR_VALUE, TAG_CLOSE, SELF_CLOSE
  - Comment: COMMENT_TEXT, COMMENT_END
  - CDATA: CDATA_TEXT, CDATA_END
  - PI: PI_TARGET, PI_TEXT, PI_END
- Comprehensive test suite covering:
  - Basic elements (simple, namespaced, empty, self-closing)
  - Attributes (double-quoted, single-quoted, multiple, on self-closing)
  - Comments (simple, whitespace preservation, dashes, between elements, empty)
  - CDATA sections (simple, angle brackets, whitespace, single bracket, empty)
  - Processing instructions (XML declaration, stylesheet PI)
  - Entity references (named, decimal char ref, hex char ref, multiple)
  - Nested and mixed content (nesting, mixed text, full document, CDATA inside element)
  - Edge cases (empty input, text only, whitespace skipping, EOF, line/column, deep nesting)
