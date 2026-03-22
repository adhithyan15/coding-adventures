# Changelog

All notable changes to the xml-lexer package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-22

### Added
- Initial release of the XML lexer package
- `tokenize_xml()` — tokenize XML text and return a list of tokens
- `create_xml_lexer()` — create a configured GrammarLexer for XML
- `xml_on_token()` — the on-token callback that drives group transitions
- Support for: tags, attributes, self-closing tags, comments, CDATA
  sections, processing instructions, entity references, character references
- First lexer wrapper to use pattern groups and callback hooks
