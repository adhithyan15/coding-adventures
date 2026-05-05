# Changelog

All notable changes to the SQL parser package will be documented in this file.

## [0.14.0] - 2026-05-04

### Added

- **`||` in `additive` grammar rule** (`sql.grammar`) — the `additive`
  production now includes `"||"` as a valid operator alongside `"+"` and `"-"`,
  enabling parsing of chained string-concatenation expressions.

- **`NATURAL` in `join_type`** (`sql.grammar`) — `join_type` now includes
  `"NATURAL"` as an alternative so `NATURAL JOIN` is a valid join syntax.

- **`USING` clause in `join_clause`** (`sql.grammar`) — `join_clause` now
  accepts either `"ON" expr` or `"USING" "(" NAME { "," NAME } ")"` as the
  join condition form.

- **Regenerated `_grammar.py`** — the pre-compiled parser grammar cache now
  reflects all three grammar changes above.

## [0.13.0] - 2026-05-04

### Added

- **`RETURNING` clause grammar** — `insert_stmt`, `update_stmt`, and
  `delete_stmt` rules now each accept an optional trailing
  `returning_clause = "RETURNING" expr { "," expr }`.  Both the canonical
  `sql.grammar` text file (used at runtime) and the pre-generated `_grammar.py`
  fallback were updated in tandem.  The parser produces an AST node named
  `returning_clause` containing one `expr` child per column expression.

## [0.12.0] - 2026-04-28

### Added

- **BLOB in primary expressions** — the `primary` grammar rule now accepts a
  `BLOB` token, enabling `x'hex'` literals in all expression positions.

## [0.11.0] - 2026-04-28

### Added — Phase 9: SQL Triggers

- **`create_trigger_stmt`** grammar rule — `CREATE TRIGGER NAME (BEFORE|AFTER)
  (INSERT|UPDATE|DELETE) ON NAME FOR EACH ROW BEGIN body END`.
- **`trigger_body_stmt`** grammar rule — alternation over `insert_stmt`,
  `update_stmt`, `delete_stmt`, `query_stmt`.  The semicolons separating
  body statements are consumed inside `create_trigger_stmt` so they don't
  conflict with the top-level `program` rule.
- **`drop_trigger_stmt`** grammar rule — `DROP TRIGGER [IF EXISTS] NAME`.
- Both added to the `statement` alternation.

## [0.10.0] - 2026-04-27

### Added — Phase 8: Window Functions (OVER / PARTITION BY)

- **`window_func_call` grammar rule** — matches `NAME "(" (STAR | [value_list]) ")" "OVER" "(" window_spec ")"`.
  Placed before `function_call` in the `primary` alternation so the PEG parser
  tries the window form first (both share the `NAME "("` prefix; window adds
  trailing `"OVER" "("`).
- **`window_spec` grammar rule** — `[ partition_clause ] [ order_clause ]`.
- **`partition_clause` grammar rule** — `"PARTITION" "BY" expr { "," expr }`.
- `_grammar.py` updated with all three new `GrammarRule` objects and the
  updated `primary` alternation.

## [0.9.0] - 2026-04-27

### Added — Phase 7: SAVEPOINT / RELEASE / ROLLBACK TO

- `savepoint_stmt`, `release_stmt`, and `rollback_to_stmt` rules added to
  `sql.grammar`.  The statement alternation places `rollback_to_stmt` before
  `rollback_stmt` so the PEG parser tries the longer form first.
- `_grammar.py` updated with three new `GrammarRule` objects and the updated
  `statement` `Alternation`.

## [0.8.0] - 2026-04-27

### Added — Phase 6: CREATE / DROP VIEW

- `create_view_stmt` and `drop_view_stmt` rules added to `sql.grammar` and
  wired into the top-level `statement` alternation.
- `_grammar.py` (compiled grammar cache) updated with the two new
  `GrammarRule` objects and updated `statement` `Alternation`.

## [0.7.0] - 2026-04-27

### Added — Phase 5b: Recursive CTEs

- `with_clause` rule extended with an optional `RECURSIVE` keyword between
  `WITH` and the first `cte_def`: `"WITH" [ "RECURSIVE" ] cte_def { "," cte_def }`.
  When present the adapter uses it as a signal to parse the CTE body as a
  recursive definition (anchor UNION [ALL] recursive) rather than a plain
  subquery.
- `_grammar.py` (auto-generated compiled grammar cache) updated to reflect
  the `with_clause` change.

## [0.6.0] - 2026-04-27

### Added — Phase 5a: Non-recursive CTEs

- `query_stmt` extended with a leading `Optional(RuleReference('with_clause'))`
  so `WITH name AS (...) SELECT ...` is now valid wherever a query is accepted.
- New `with_clause` rule: `"WITH" cte_def { "," cte_def }` — allows one or
  more comma-separated CTE definitions.
- New `cte_def` rule: `NAME "AS" "(" query_stmt ")"` — each CTE is a named
  subquery; the body is itself a full `query_stmt` supporting all SELECT
  features.

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

