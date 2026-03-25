# Changelog — sql-lexer (Go)

## [0.1.0] — 2026-03-23

### Added
- `CreateSQLLexer(source string)` — loads `sql.tokens` grammar and returns a configured `GrammarLexer`
- `TokenizeSQL(source string)` — convenience one-shot tokenization function
- Case-insensitive keyword matching via `# @case_insensitive true` in `sql.tokens`; all keyword values normalized to uppercase on emit
- Token aliases: `STRING_SQ → STRING`, `QUOTED_ID → NAME`, `NEQ_ANSI → NOT_EQUALS`
- Skip patterns: whitespace, `--` line comments, `/* block */` comments
- Full SQL keyword list: SELECT, FROM, WHERE, GROUP, BY, HAVING, ORDER, LIMIT, OFFSET, INSERT, INTO, VALUES, UPDATE, SET, DELETE, CREATE, DROP, TABLE, IF, EXISTS, NOT, AND, OR, NULL, IS, IN, BETWEEN, LIKE, AS, DISTINCT, ALL, UNION, INTERSECT, EXCEPT, JOIN, INNER, LEFT, RIGHT, OUTER, CROSS, FULL, ON, ASC, DESC, TRUE, FALSE, CASE, WHEN, THEN, ELSE, END, PRIMARY, KEY, UNIQUE, DEFAULT
- 12 unit tests covering: SELECT tokenization, case-insensitive keywords, numbers, strings, operators, punctuation, comments (line and block), qualified names, quoted identifiers, full statements, NULL/TRUE/FALSE, and `CreateSQLLexer`
