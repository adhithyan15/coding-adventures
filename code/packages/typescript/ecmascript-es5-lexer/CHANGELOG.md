# Changelog

## 0.1.0 — 2026-04-05

- Initial release
- Tokenizes ECMAScript 5 (2009) source code using the grammar-driven lexer
- Loads `es5.tokens` grammar file
- Supports ES5-specific features: `debugger` keyword, reduced reserved word list
- Inherits all ES3 features: strict equality, error handling keywords, regex
- Thin wrapper around `@coding-adventures/lexer`
