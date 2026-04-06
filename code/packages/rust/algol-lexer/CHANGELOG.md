# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-04-06

### Added

- Initial release of the ALGOL 60 lexer crate.
- `create_algol_lexer()` factory function returning a `GrammarLexer` configured for ALGOL 60.
- `tokenize_algol()` convenience function returning `Vec<Token>` directly.
- Loads the `algol.tokens` grammar file at runtime from the shared `grammars/` directory.
- Supports all ALGOL 60 token types: IDENT, INTEGER_LIT, REAL_LIT, STRING_LIT, ASSIGN, POWER, LEQ, GEQ, NEQ, PLUS, MINUS, STAR, SLASH, CARET, EQ, LT, GT, LPAREN, RPAREN, LBRACKET, RBRACKET, SEMICOLON, COMMA, COLON.
- Keywords promoted from IDENT: begin, end, if, then, else, for, do, step, until, while, goto, switch, procedure, integer, real, boolean, string, array, own, label, value, true, false, not, and, or, impl, eqv, div, mod.
- Whitespace (spaces, tabs, newlines, carriage returns) is silently skipped.
- Comments (`comment ... ;`) are silently consumed via the skip pattern.
- 33 unit tests covering integer literals, real literals (decimal, exponent, negative exponent), string literals, identifiers, all keywords, keyword boundary disambiguation, all operators (ASSIGN vs COLON, POWER vs STAR), relational operators, delimiters, comment skipping, whitespace skipping, multi-token expressions, and the factory function.
