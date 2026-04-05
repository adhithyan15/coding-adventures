# Changelog

## 0.1.0 — 2026-04-05

- Initial release
- Parses ECMAScript 5 (2009) source code into ASTs using the grammar-driven parser
- Loads `es5.grammar` grammar file
- Supports ES5-specific features: debugger statement, getter/setter properties in object literals
- Inherits all ES3 grammar rules (try/catch/finally, throw, strict equality, instanceof)
- Thin wrapper around `@coding-adventures/parser`
