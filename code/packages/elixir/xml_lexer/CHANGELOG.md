# Changelog

## 0.1.0 — 2026-03-21

### Added
- `XmlLexer.tokenize/1` — tokenize XML source code using pattern groups and callback hooks
- `XmlLexer.create_lexer/0` — parse xml.tokens grammar
- `XmlLexer.xml_on_token/2` — callback function that drives group switching for context-sensitive XML lexing
- Grammar caching via `persistent_term` for repeated use
- 30+ tests covering basic tags, attributes, self-closing tags, comments, CDATA sections, processing instructions, entity references, nested/mixed content, edge cases, and callback actions
