# Changelog

All notable changes to the SQL parser package will be documented in this file.

## [0.5.0] - 2026-04-27

### Added — Phase 4b: FOREIGN KEY constraints

- `col_constraint` alternation extended with `REFERENCES NAME [ ( NAME ) ]` —
  the optional column list `(col)` is wrapped in an `Optional(Group(...))` so
  both `REFERENCES customers` and `REFERENCES customers(id)` parse correctly.

## [0.4.0] - 2026-04-27

### Added — Phase 4a: CHECK constraints

- `col_constraint` alternation extended with `CHECK ( expr )` — the parser
  now accepts per-column CHECK constraints and includes the expression node
  in the parse tree under the `col_constraint` → `expr` rule reference.

## [0.3.0] - 2026-04-27

### Added
- `alter_table_stmt` rule — `ALTER TABLE NAME ADD [COLUMN] col_def` — added to
  the `statement` alternation and compiled into `_grammar.py`.

## [0.2.0] - 2026-04-21

### Changed

- **Grammar: `join_clause` ON clause is now optional** — changed from
  `join_type "JOIN" table_ref "ON" expr` to
  `join_type "JOIN" table_ref [ "ON" expr ]`.  This is required for
  `CROSS JOIN` which has no ON predicate.  The change is backwards-compatible:
  all existing `INNER JOIN … ON …` queries continue to work.

- **Grammar: `table_ref` now supports derived tables** — extended to accept
  `"(" query_stmt ")" "AS" NAME` (a parenthesised subquery with mandatory
  alias) in addition to the existing plain `table_name [ "AS" NAME ]` form.
  Derived tables can appear in the primary FROM position and in JOIN targets.

- `_grammar.py` (auto-generated compiled grammar cache) updated to reflect both
  of the above grammar changes.  Note: `_grammar.py` is generated from
  `code/grammars/sql.grammar`; the canonical source of truth is the `.grammar`
  file.

## [0.1.0] - 2026-03-23

### Added
- Initial release of the SQL parser thin wrapper.
- `parse_sql()` function for one-step parsing of SQL text into ASTs.
- `create_sql_parser()` factory for creating configured `GrammarParser` instances.
- Full ANSI SQL subset grammar support: SELECT, INSERT, UPDATE, DELETE,
  CREATE TABLE, DROP TABLE.
- SELECT clause features: `*`, multiple columns, `AS` aliases, `DISTINCT`, `ALL`.
- WHERE clause support with comparisons, `AND`/`OR`/`NOT`, `BETWEEN`, `IN`,
  `LIKE`, `IS NULL`, `IS NOT NULL`.
- JOIN support: `INNER JOIN`, `LEFT JOIN`, `RIGHT JOIN`, `FULL JOIN`, `CROSS JOIN`.
- Aggregate support: `GROUP BY`, `HAVING`, `ORDER BY` (ASC/DESC), `LIMIT`, `OFFSET`.
- CREATE TABLE with `IF NOT EXISTS`, column constraints (`NOT NULL`, `PRIMARY KEY`,
  `UNIQUE`, `DEFAULT`).
- DROP TABLE with `IF EXISTS`.
- Multiple semicolon-separated statements in a single `parse_sql()` call.
- Expression grammar: arithmetic, logical operators, function calls, column refs.
- Case-insensitive keyword matching (delegated to the SQL lexer).
- Produces generic `ASTNode` trees — root rule_name is `"program"`.
- `py.typed` marker for PEP 561 typing support.
- `_sql_grammar_path` module-level override for test error-path coverage.

