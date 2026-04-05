# Changelog

## 0.1.0 — 2026-04-05

- Initial release
- Parses ECMAScript 3 (1999) source code into ASTs using the grammar-driven parser
- Loads `es3.grammar` grammar file
- Supports ES3-specific features: try/catch/finally, throw, strict equality, instanceof
- Inherits all ES1 grammar rules
- Thin wrapper around `@coding-adventures/parser`
