# Changelog — CodingAdventures::JavascriptLexer (Perl)

All notable changes to this package are documented here.

## [0.01] — 2026-03-29

### Added

- Initial implementation of `CodingAdventures::JavascriptLexer`.
- `tokenize($source)` — tokenizes a JavaScript string using rules compiled
  from the shared `javascript.tokens` grammar file.
- Grammar is read from `code/grammars/javascript.tokens` once and cached in
  package-level variables (`$_grammar`, `$_rules`, `$_skip_rules`).
- Path navigation uses `File::Basename::dirname` and `File::Spec::rel2abs`
  relative to `__FILE__`, climbing 5 directory levels to the repo root.
- Skip patterns (whitespace) are consumed silently; no WHITESPACE tokens
  are emitted.
- Token types: NAME, NUMBER, STRING; keyword types: LET, CONST, VAR, IF,
  ELSE, WHILE, FOR, DO, FUNCTION, RETURN, CLASS, IMPORT, EXPORT, FROM, AS,
  NEW, THIS, TYPEOF, INSTANCEOF, TRUE, FALSE, NULL, UNDEFINED; operator
  types: STRICT_EQUALS, STRICT_NOT_EQUALS, EQUALS_EQUALS, NOT_EQUALS,
  LESS_EQUALS, GREATER_EQUALS, ARROW, EQUALS, PLUS, MINUS, STAR, SLASH,
  LESS_THAN, GREATER_THAN, BANG; delimiter types: LPAREN, RPAREN, LBRACE,
  RBRACE, LBRACKET, RBRACKET, COMMA, COLON, SEMICOLON, DOT.
- Alias resolution: definitions with `-> ALIAS` syntax emit the alias name.
- Line and column tracking for all tokens.
- `die` with a descriptive "LexerError" message on unexpected input.
- `t/00-load.t` — smoke test that the module loads and has a VERSION.
- `t/01-basic.t` — comprehensive test suite covering all keywords,
  identifiers, numbers, strings, operators, punctuation, arrow functions,
  composite expressions (if/else, function declarations, class declarations,
  method calls), whitespace, position tracking, and error handling.
- `BUILD` and `BUILD_windows` scripts.
