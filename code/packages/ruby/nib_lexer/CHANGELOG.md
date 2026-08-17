# Changelog

All notable changes to `coding_adventures_nib_lexer` will be documented in this file.

## [0.1.2] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `create_nib_lexer` now loads the pre-compiled `_grammar.rb` (via `GrammarTools::CompiledLoader`, newly generated from `nib.tokens`) instead of reading and parsing the `.tokens` file from `code/grammars/` on every call. The old code walked out of the installed gem's own directory to a monorepo-relative path that a published gem does not ship, so a real `gem install` + first use would raise `Errno::ENOENT`.

## [0.1.1] - 2026-03-31

### Fixed

- STRING token values now preserve their original case even when SQL's
  case-insensitive grammar is active. Previously, `'Alice'` in a SQL string
  literal would produce `STRING("alice")` because the lexer lowercased the
  entire source for case-insensitive keyword matching. This is fixed in the
  underlying `ruby/lexer` package (GrammarLexer now stores an `@original_source`
  and extracts string bodies from it). Tests `test_insert_statement` and
  `test_update_statement` now pass.

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
