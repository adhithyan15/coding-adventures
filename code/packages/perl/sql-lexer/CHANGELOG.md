# Changelog — CodingAdventures::SqlLexer (Perl)

All notable changes to this package will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This package uses [Semantic Versioning](https://semver.org/).

## [0.01] - 2026-03-29

### Added

- Initial implementation of the SQL lexer Perl package.
- `tokenize($source)` class method — tokenizes a SQL source string using
  the grammar-driven infrastructure, returning an arrayref of token hashrefs.
  Each hashref has keys: `type`, `value`, `line`, `col`.
- Grammar loading with caching — the `sql.tokens` file is read and parsed
  once per process; subsequent calls reuse the cached `TokenGrammar`.
- Path navigation — locates `sql.tokens` by climbing 5 directory levels
  from `lib/CodingAdventures/` to the `code/` repo root, then descending
  into `grammars/`.
- Pattern compilation — both regex and literal token definitions are compiled
  into `\G`-anchored `qr//` patterns for efficient position-based matching.
- Skip-first algorithm — skip patterns (whitespace, line comments, block
  comments) are tried at each position before token patterns.
- Full test suite covering:
  - Module loading and VERSION check (t/00-load.t)
  - Empty and trivial inputs (empty, whitespace-only, comment-only) (t/01-basic.t)
  - SELECT * FROM users WHERE id = 1
  - SELECT with column list, comparison operators, ORDER BY, LIMIT, DISTINCT
  - Case-insensitive keyword matching (select → SELECT, from → FROM)
  - INSERT INTO ... VALUES ... with and without column list
  - UPDATE ... SET ... WHERE ...
  - DELETE FROM ... WHERE ...
  - Single-quoted string literals
  - Integer and decimal numeric literals
  - NULL, TRUE, FALSE literals and IS NULL / IS NOT NULL
  - All comparison operators: =, !=, <>, <, >, <=, >=
  - All arithmetic operators: +, -, *, /, %
  - Delimiters: ( ) , ; .
  - Line comments (-- ...) and block comments (/* ... */)
  - JOIN clauses (INNER JOIN, LEFT JOIN)
  - BETWEEN...AND, LIKE, IN list
  - GROUP BY, HAVING, ORDER BY
  - CREATE TABLE statement
  - Token position tracking (line, col)
  - Error on unexpected character
- `Makefile.PL` with correct PREREQ_PM entries.
- `cpanfile` listing runtime and test dependencies.
- `BUILD` script installing all transitive deps leaf-to-root via cpanm.
- `BUILD_windows` skipping Perl tests (not supported on Windows CI).
