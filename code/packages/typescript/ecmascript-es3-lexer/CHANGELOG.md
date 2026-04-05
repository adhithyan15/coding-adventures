# Changelog

## 0.1.0 — 2026-04-05

- Initial release
- Tokenizes ECMAScript 3 (1999) source code using the grammar-driven lexer
- Loads `es3.tokens` grammar file
- Supports ES3-specific features: strict equality (===, !==), try/catch/finally/throw keywords, instanceof, regex literals
- Thin wrapper around `@coding-adventures/lexer`
