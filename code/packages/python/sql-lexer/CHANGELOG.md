# Changelog

All notable changes to the SQL lexer package will be documented in this file.

## [0.27.0] - 2026-05-23

### Added

- ``AUTOINCREMENT`` keyword recognised by the lexer.  Used by SQLite's
  ``CREATE TABLE t (id INTEGER PRIMARY KEY AUTOINCREMENT)`` form,
  which previously failed to parse.
- Regenerated `_grammar.py` to include the new keyword.

## [0.26.0] - 2026-05-23

### Added

- ``INDEXED`` keyword recognised by the lexer.  Required by the new
  ``INDEXED BY <name>`` and ``NOT INDEXED`` table-source query hints
  parsed in sql-parser 0.38.
- Regenerated `_grammar.py` to include the new keyword.

## [0.25.0] - 2026-05-23

### Added

- ``RENAME`` keyword recognised by the lexer.  Required by the new
  ``ALTER TABLE … RENAME TO`` / ``RENAME [COLUMN]`` forms parsed in
  sql-parser 0.37.
- Regenerated `_grammar.py` to include the new keyword.

## [0.24.0] - 2026-05-21

### Added

- Recognise SQLite hex integer literals (`0x1F`, `0XDEADBEEF`).  The
  new `HEX_INT` token (`/0[xX][0-9A-Fa-f]+/`) is declared *before*
  `NUMBER` so the longest-match rule wins — otherwise the lexer would
  emit `0` as a NUMBER and leave `xFF` to be parsed as a NAME, the way
  it did before this change.  `HEX_INT` aliases to `NUMBER` so the
  parser grammar keeps using a single literal-integer terminal
  everywhere.

## [0.23.0] - 2026-05-21

### Added

- Five new bitwise operator tokens matching SQLite syntax:
  - `BIT_AND_OP` (`&`), `BIT_OR_OP` (`|`), `BIT_NOT_OP` (`~`) — single-character.
  - `SHIFT_LEFT` (`<<`), `SHIFT_RIGHT` (`>>`) — declared *before* the
    single-character `<` and `>` so the longest-match rule picks them up
    instead of producing two adjacent comparison tokens.
- Regenerated `_grammar.py` table so downstream consumers see the new
  token kinds without rebuilding grammar-tools.

## [0.22.0] - 2026-05-19

### Added

- **``NULLS`` keyword** for the SQLite 3.30+ ``ORDER BY ... NULLS
  FIRST | NULLS LAST`` clause (``sql.tokens``, ``_grammar.py``).

  ``FIRST`` and ``LAST`` are intentionally NOT added as keywords —
  they would conflict with common column names like ``first_name``
  and ``last``.  The grammar uses ``"NULLS" NAME`` and the adapter
  validates that the NAME is FIRST or LAST.

## [0.21.0] - 2026-05-19

### Added

- **`MATERIALIZED` keyword** (`sql.tokens`, `_grammar.py`) — added to
  the keyword list so the parser can recognise the SQLite 3.35+ CTE
  optimizer hint ``WITH cte AS [NOT] MATERIALIZED (…)``.  The lexer's
  generated cache was regenerated via ``grammar-tools compile-tokens``.

## [0.20.0] - 2026-05-18

### Added

- **`STRICT` and `WITHOUT` keywords** (`sql.tokens`, `_grammar.py`) —
  added to the keyword list so the parser can recognise the
  `CREATE TABLE … STRICT` (SQLite 3.37+) and `… WITHOUT ROWID`
  (SQLite 3.8.2+) table options.  Mini-sqlite ignores both, but accepting
  the syntax unlocks ORM/migration code.

  Note: `ROWID` is intentionally NOT promoted to a keyword — it remains
  usable as an ordinary column name (`SELECT rowid FROM t`), and the
  parser matches it as a NAME inside the `WITHOUT NAME` table option.

## [0.19.0] - 2026-05-18

### Added

- **`ATTACH`, `DETACH`, `DATABASE` keywords** (`sql.tokens`, `_grammar.py`)
  — added to the keyword list so the parser can recognise the SQLite
  `ATTACH [DATABASE] expr AS name` and `DETACH [DATABASE] name` statements.
  Mini-sqlite no-ops these (single-database engine), but accepting the
  syntax unlocks ORM/migration code.

## [0.18.0] - 2026-05-18

### Added

- **`COLLATE` keyword** (`sql.tokens`, `_grammar.py`) — added to the keyword
  list so the parser can recognise `COLLATE NOCASE` / `COLLATE BINARY` /
  `COLLATE RTRIM` clauses in `CREATE INDEX … ON t(col COLLATE …)` and
  similar contexts.  Mini-sqlite ignores the collation name (only BINARY
  is implemented), but accepting the syntax unlocks ORM/migration code.

## [0.17.0] - 2026-05-17

### Added

- **`JSON_ARROW` (`->`) and `JSON_ARROW_TEXT` (`->>`) tokens** (`sql.tokens`,
  `_grammar.py`) — SQLite 3.38+ JSON path-shortcut operators.  `->>` is
  defined first so the longest-match lexing rule picks it up correctly when
  followed by `>`.

## [0.16.0] - 2026-05-17

### Changed

- **`ESCAPE` keyword** (`sql.tokens`, `_grammar.py`) — promoted to a reserved
  SQL keyword so the parser can recognise the `LIKE … ESCAPE 'c'` clause.

- **`STRING_SQ` pattern simplified to SQLite semantics** (`sql.tokens`,
  `_grammar.py`) — the old pattern allowed backslash escapes (`\\.`), which
  meant `'a\\_b'` was lexed as the three characters `a_b` after escape
  processing.  SQLite treats backslashes as **literal characters** inside
  string literals — only `''` (doubled apostrophe) is an escape.  The new
  pattern `'(\'\'|[^\'])*\'` matches SQLite exactly.

- **`escape_mode = 'none'`** (`_grammar.py`) — disables the base
  `GrammarLexer`'s default JSON-style escape processing on STRING tokens.
  Combined with the new pattern, string literals now arrive at the parser
  with their content untouched (modulo doubled-quote folding done in the
  adapter).

These changes are essential for `LIKE 'a\\_b' ESCAPE '\\'` to work: the
backslash must survive the lexer so the LIKE matcher can see it as an
escape character.

## [0.15.0] - 2026-05-16

### Added

- **`QUOTED_ID_DQ` token** (`sql.tokens`, `_grammar.py`) — double-quoted
  identifiers `"name"` are now lexed and aliased to `NAME`, matching SQLite's
  ANSI SQL behaviour.  The pattern `"([^"]|"")*"` handles embedded
  double-quote escaping (`""` → one `"` inside the identifier).

- **Double-quote stripping in `tokenize_sql`** — the post-processing loop in
  `tokenize_sql` strips the surrounding `"` characters from `QUOTED_ID_DQ`
  tokens and un-escapes any `""` sequences, so callers receive a clean string
  value: `"my col"` → `NAME('my col')`.

### Fixed

- **`sql.tokens` keyword list cleanup** — `TEMP` and `TEMPORARY` were
  erroneously added to the keyword section in a prior revision; they have been
  removed.  These words are not reserved in our grammar because `temp` is a
  common table name in SQLite compatibility scripts.  The `CREATE TEMP TABLE`
  normalisation now happens at the engine layer (mini-sqlite) rather than in
  the lexer.

## [0.14.0] - 2026-05-04

### Added

- **`REPLACE`, `IGNORE`, `ABORT`, `FAIL` keywords** (`sql.tokens`) — four new
  SQL conflict-resolution keywords added to the keyword list to support the
  `INSERT OR REPLACE`, `INSERT OR IGNORE`, `INSERT OR ABORT`, and
  `INSERT OR FAIL` syntax, as well as `REPLACE INTO` (shorthand for
  `INSERT OR REPLACE INTO`).

- **Regenerated `_grammar.py`** — the pre-compiled token grammar cache now
  includes all four new keywords.

## [0.13.0] - 2026-05-04

### Added

- **`CONCAT_OP = "||"` token** (`sql.tokens`) — SQL string-concatenation
  operator.  Defined before single-character operators so longest-match lexing
  picks `||` as one token rather than two `|` tokens.  Aliased to `CONCAT_OP`
  (no alias needed; it is already a distinct token type).

- **`NATURAL` and `USING` keywords** (`sql.tokens`) — added to the keyword
  list to support `NATURAL JOIN` and `JOIN … USING (col)` syntax.

- **Regenerated `_grammar.py`** — the pre-compiled token grammar cache now
  includes the `CONCAT_OP` token definition and `NATURAL` / `USING` in the
  keyword list.

## [0.12.0] - 2026-05-04

### Added

- **`RETURNING` keyword** — added `'RETURNING'` to the `keywords` list in
  `_grammar.py` and to the `keywords:` section of `sql.tokens`.  The lexer now
  emits a `KEYWORD` token with value `"RETURNING"` for any `RETURNING`
  occurrence, enabling the parser to recognise the DML RETURNING clause.

## [0.11.0] - 2026-04-28

### Added

- **BLOB_HEX token** — regex pattern `[xX]'[0-9A-Fa-f]*'` lexed as token
  type `BLOB`. Placed before the `NAME` token definition so that `x'...'`
  is captured as a single token rather than `NAME` + `STRING`.

## [0.10.0] - 2026-04-28

### Added — Phase 9: SQL Triggers

- **6 new keyword tokens** — `TRIGGER`, `BEFORE`, `AFTER`, `FOR`, `EACH`, `ROW`
  added to `sql.tokens`.  `NEW` and `OLD` remain `NAME` tokens (not keywords)
  so they can still be used as table or column aliases without quoting.

## [0.9.0] - 2026-04-27

### Added — Phase 8: Window Functions (OVER / PARTITION BY)

- `OVER` and `PARTITION` registered as SQL keywords in `sql.tokens` and the
  compiled `_grammar.py`, so these words tokenize as KEYWORD rather than NAME
  when they appear in window-function expressions.

## [0.8.0] - 2026-04-27

### Added — Phase 7: SAVEPOINT / RELEASE / ROLLBACK TO

- `SAVEPOINT`, `RELEASE`, and `TO` registered as SQL keywords in `sql.tokens`
  and the compiled `_grammar.py`, so these words tokenize as KEYWORD rather
  than NAME when they appear at the start of a savepoint statement.

## [0.7.0] - 2026-04-27

### Added — Phase 6: CREATE / DROP VIEW

- `VIEW` registered as a SQL keyword in `sql.tokens` and the compiled
  `_grammar.py`, enabling `CREATE VIEW` and `DROP VIEW` to tokenize `VIEW`
  as KEYWORD rather than NAME.

## [0.6.0] - 2026-04-27

### Added — Phase 5b: Recursive CTEs

- `RECURSIVE` registered as a SQL keyword in `sql.tokens` and the compiled
  `_grammar.py`, enabling `WITH RECURSIVE name AS (...)` to tokenize `RECURSIVE`
  as KEYWORD rather than NAME.

## [0.5.0] - 2026-04-27

### Added — Phase 5a: Non-recursive CTEs

- `WITH` registered as a SQL keyword so `WITH name AS (...)` common table
  expressions tokenize `WITH` as KEYWORD rather than NAME.

## [0.4.0] - 2026-04-27

### Added — Phase 4b: FOREIGN KEY constraints

- `REFERENCES` registered as a SQL keyword so `REFERENCES table(col)` column
  constraints tokenize as KEYWORD rather than NAME.

## [0.3.0] - 2026-04-27

### Added — Phase 4a: CHECK constraints

- `CHECK` registered as a SQL keyword in `sql.tokens` and the compiled
  `_grammar.py`, enabling `CHECK (expr)` column constraints to tokenize as
  KEYWORD rather than NAME.

## [0.2.0] - 2026-04-27

### Added
- `ALTER`, `ADD`, `COLUMN` registered as SQL keywords in `sql.tokens` and
  the compiled `_grammar.py`, enabling ALTER TABLE to tokenize correctly.

## [0.1.0] - 2026-03-23

### Added
- Initial release of the SQL lexer thin wrapper.
- `tokenize_sql()` function for one-step tokenization of ANSI SQL text.
- `create_sql_lexer()` factory for creating configured `GrammarLexer` instances.
- Full ANSI SQL token support: KEYWORD, NAME, NUMBER, STRING, all comparison
  and arithmetic operators, and punctuation (, ; . ( )).
- Case-insensitive keyword normalization via `# @case_insensitive true` in
  `sql.tokens`: `select`, `SELECT`, and `Select` all produce `KEYWORD("SELECT")`.
- Both `!=` and `<>` produce `NOT_EQUALS` tokens (NEQ_ANSI aliased).
- Compound operators `<=`, `>=` matched as single tokens (longest-match rule).
- Single-quoted string literals aliased to STRING (quotes stripped).
- Backtick-quoted identifiers aliased to NAME (backticks preserved in value).
- `--` line comments and `/* */` block comments silently skipped.

