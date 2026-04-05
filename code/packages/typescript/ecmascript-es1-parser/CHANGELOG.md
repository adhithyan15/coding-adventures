# Changelog

## 0.1.0 — 2026-04-05

- Initial release
- Parses ECMAScript 1 (1997) source code into ASTs using the grammar-driven parser
- Loads `es1.grammar` grammar file
- Supports var declarations, function declarations/expressions, all 14 ES1 statement types
- Full expression precedence chain from comma to primary
- Thin wrapper around `@coding-adventures/parser`
