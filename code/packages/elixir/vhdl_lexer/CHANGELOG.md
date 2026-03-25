# Changelog

## 0.1.0 — 2026-03-22

### Added
- `VhdlLexer.tokenize/1` — tokenize VHDL source code with case normalization
- `VhdlLexer.create_lexer/0` — parse vhdl.tokens grammar
- Post-tokenization case normalization: NAME and KEYWORD values are lowercased
  - Uppercase keywords (ENTITY, ARCHITECTURE) correctly become KEYWORD tokens
  - Mixed-case identifiers are normalized to lowercase
  - Extended identifiers (\name\) preserve original case
- Grammar caching via `persistent_term` for repeated use
- 45+ tests covering entity declarations, architecture, case insensitivity,
  character literals, bit strings, operators, keywords, comments, and
  complete VHDL snippets
