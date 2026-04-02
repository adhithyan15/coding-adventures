# Changelog — sql-lexer

All notable changes to this package are documented here.

## [0.2.0] — 2026-03-31

### Changed

- Switch from runtime `readFileSync` grammar loading to pre-compiled `_grammar.ts` import
- Remove `fs`, `path`, `url` Node.js built-in dependencies
- Package now works in browsers, edge runtimes, and any JavaScript environment
- No file system access needed at runtime — grammar is embedded as a TypeScript object

## [0.1.0] — 2026-03-23

### Added

- Initial implementation of `tokenizeSQL(source: string): Token[]`
- `createSQLLexer(source: string): Token[]` factory function (alias for `tokenizeSQL`)
- Grammar-driven tokenization via `sql.tokens` grammar file
- Case-insensitive keyword matching (`select` → `KEYWORD("SELECT")`)
- Single-quoted string literals (`'hello'` → `STRING("hello")`, quotes stripped)
- Backtick-quoted identifiers (`` `col` `` → `NAME("\`col\`")`, backticks retained)
- Multi-character operator support: `!=`, `<>` (both → `NOT_EQUALS`), `<=`, `>=`
- Comparison operators: `=`, `<`, `>`, `<=`, `>=`, `!=`, `<>`
- Arithmetic operators: `+`, `-`, `*`, `/`, `%`
- Punctuation: `(`, `)`, `,`, `;`, `.`
- SQL comment skipping: line comments (`-- ...`) and block comments (`/* ... */`)
- SQL keywords: SELECT, FROM, WHERE, GROUP BY, HAVING, ORDER BY, LIMIT, OFFSET,
  INSERT INTO, VALUES, UPDATE, SET, DELETE, CREATE TABLE, DROP TABLE, IF EXISTS,
  NULL, TRUE, FALSE, AND, OR, NOT, IS, IN, BETWEEN, LIKE, AS, DISTINCT, ALL,
  JOIN (INNER, LEFT, RIGHT, FULL, CROSS, OUTER), ON, ASC, DESC, CASE, WHEN,
  THEN, ELSE, END, PRIMARY KEY, UNIQUE, DEFAULT, UNION, INTERSECT, EXCEPT
- Comprehensive test suite with 95%+ coverage
- `package.json`, `tsconfig.json`, `BUILD`, `README.md`
