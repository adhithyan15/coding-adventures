# Changelog

All notable changes to `coding_adventures_xml_lexer` will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- Initial release
- `CodingAdventures::XmlLexer.tokenize(source)` method that tokenizes XML text
- `CodingAdventures::XmlLexer.create_xml_lexer(source)` factory for configured GrammarLexer
- `XML_ON_TOKEN` callback proc for pattern group switching
- Loads `xml.tokens` grammar file and delegates to `GrammarLexer` with on-token callback
- Pattern group switching for context-sensitive lexing:
  - **default** group: TEXT, ENTITY_REF, CHAR_REF, tag/comment/CDATA/PI openers
  - **tag** group: TAG_NAME, ATTR_EQUALS, ATTR_VALUE, TAG_CLOSE, SELF_CLOSE
  - **comment** group: COMMENT_TEXT, COMMENT_END (skip disabled)
  - **cdata** group: CDATA_TEXT, CDATA_END (skip disabled)
  - **pi** group: PI_TARGET, PI_TEXT, PI_END (skip disabled)
- Supports namespace prefixes in tag names (e.g., `ns:tag`)
- Supports single and double quoted attribute values
- Full test suite with SimpleCov coverage >= 80%
