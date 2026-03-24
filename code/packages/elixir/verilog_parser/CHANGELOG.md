# Changelog

## 0.1.0 — 2026-03-20

### Added
- `VerilogParser.parse/1` — parse Verilog source code into an AST
- `VerilogParser.create_parser/0` — parse verilog.grammar
- Grammar caching via `persistent_term` for repeated use
- 21 tests covering primitives, objects, arrays, nested structures, RFC 8259 example, whitespace, and errors
