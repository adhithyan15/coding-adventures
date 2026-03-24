# Changelog

## 0.1.0 — 2026-03-20

### Added
- `VhdlParser.parse/1` — parse VHDL source code into an AST
- `VhdlParser.create_parser/0` — parse vhdl.grammar
- Grammar caching via `persistent_term` for repeated use
- 21 tests covering primitives, objects, arrays, nested structures, RFC 8259 example, whitespace, and errors
