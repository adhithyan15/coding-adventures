# Changelog

## 0.1.0 — 2026-03-20

### Added

- **Lisp parser** — thin wrapper around grammar-tools GrammarParser
- Loads `lisp.grammar` for grammar-driven parsing
- `create_lisp_parser()` factory and `parse_lisp()` convenience function
- 6 grammar rules: program, sexpr, atom, list, list_body, quoted
- Supports dotted pairs via DOT token in list_body
- 24 tests, 100% coverage
