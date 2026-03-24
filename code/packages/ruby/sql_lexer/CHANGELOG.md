# Changelog

All notable changes to `coding_adventures_sql_lexer` will be documented in this file.

## [0.1.0] - 2026-03-23

### Added
- Initial release
- `CodingAdventures::SqlLexer.create_sql_lexer(source)` method that returns a configured GrammarLexer
- `CodingAdventures::SqlLexer.tokenize_sql(source)` method that tokenizes SQL text
- Loads `sql.tokens` grammar file and delegates to `GrammarLexer`
- Supports ANSI SQL subset keywords: SELECT, FROM, WHERE, GROUP, BY, HAVING, ORDER, LIMIT,
  OFFSET, INSERT, INTO, VALUES, UPDATE, SET, DELETE, CREATE, DROP, TABLE, IF, EXISTS, NOT,
  AND, OR, NULL, IS, IN, BETWEEN, LIKE, AS, DISTINCT, ALL, UNION, INTERSECT, EXCEPT, JOIN,
  INNER, LEFT, RIGHT, OUTER, CROSS, FULL, ON, ASC, DESC, TRUE, FALSE, CASE, WHEN, THEN,
  ELSE, END, PRIMARY, KEY, UNIQUE, DEFAULT
- Case-insensitive keyword matching via `@case_insensitive true` in sql.tokens; all keyword
  values normalized to uppercase (e.g., "select" -> KEYWORD "SELECT")
- NUMBER tokens for integers and decimals
- STRING tokens for single-quoted literals (quotes stripped)
- NAME tokens for unquoted identifiers and backtick-quoted identifiers (backticks kept)
- Operators: EQUALS, NOT_EQUALS (both != and <>), LESS_THAN, GREATER_THAN,
  LESS_EQUALS, GREATER_EQUALS, PLUS, MINUS, STAR, SLASH, PERCENT
- Punctuation: LPAREN, RPAREN, COMMA, SEMICOLON, DOT
- Line comment skipping (-- to end of line)
- Block comment skipping (/* ... */, may span multiple lines)
- Full test suite with SimpleCov coverage >= 80%
