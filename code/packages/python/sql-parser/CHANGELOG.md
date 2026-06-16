# Changelog

All notable changes to the SQL parser package will be documented in this file.

## [0.45.0] - 2026-06-16

### Added

- **Named WINDOW clause grammar** — ``sql.grammar`` changes:

  - ``WINDOW`` added to ``sql.tokens`` keyword list so the lexer
    classifies it as a keyword token rather than a plain name.
  - ``window_func_call`` modified to accept either an inline spec or a
    name reference after ``OVER``::

        window_func_call = NAME "(" ( STAR | [ value_list ] ) ")" "OVER"
                           ( "(" window_spec ")" | window_name_ref ) ;
        window_name_ref  = NAME ;

  - New ``window_clause`` rule::

        window_clause = "WINDOW" NAME "AS" "(" window_spec ")"
                        { "," NAME "AS" "(" window_spec ")" } ;

  - ``select_stmt`` extended with ``[ window_clause ]`` between
    ``having_clause`` and ``order_clause``.

  ``_grammar.py`` regenerated from the updated grammar source via
  ``grammar-tools compile-grammar``.

## [0.44.0] - 2026-06-15

### Added

- **Row-value comparison grammar** — the ``comparison`` rule in
  ``code/grammars/sql.grammar`` gained three new PEG alternatives that
  fire before the scalar ``collated`` form:

      comparison = row_value cmp_op row_value
                 | row_value "NOT" "IN" "(" row_value_list ")"
                 | row_value "IN" "(" row_value_list ")"
                 | collated [ ... ] ;

  A new ``row_value_list`` rule is also defined to support the IN variant::

      row_value_list = row_value { "," row_value } ;

  ``_grammar.py`` was regenerated from the updated grammar source via
  ``grammar-tools compile-grammar``.  The ordering ensures the PEG parser
  tries the row-value form first when it sees ``(`` so that scalar
  regressions are unaffected.

## [0.43.0] - 2026-05-23

### Changed

- ``limit_clause`` now accepts SQLite's signed counts and
  MySQL-compatible comma syntax::

      limit_clause  = "LIMIT" signed_number
                      [ "OFFSET" signed_number | "," signed_number ] ;
      signed_number = [ "-" ] NUMBER ;

  ``LIMIT -1`` (no limit), ``LIMIT 5 OFFSET -2`` (negative offset
  treated as zero), and ``LIMIT m, n`` (≡ ``LIMIT n OFFSET m``)
  now parse cleanly.  Regenerated ``_grammar.py`` cache via
  ``grammar-tools``.

## [0.42.0] - 2026-05-23

### Changed

- ``comparison`` rule now accepts SQLite's general NULL-safe equality
  forms ``"IS" collated`` and ``"IS" "NOT" collated`` (in addition
  to the existing ``"IS" "NULL"`` / ``"IS" "NOT" "NULL"`` /
  ``"IS" [NOT] "DISTINCT" "FROM" collated`` alternatives).  The new
  forms are listed *after* the specific NULL / DISTINCT shapes so
  the PEG parser still matches those first.  Regenerated
  ``_grammar.py`` cache via ``grammar-tools``.

- Unary ``+`` prefix accepted as a no-op identity in the ``unary``
  rule (see mini-sqlite CHANGELOG 2.6.0 for the end-to-end change).
  This was inadvertently omitted from the previous bump; pinning the
  version here so the parser ships with the matching grammar.

## [0.41.0] - 2026-05-23

### Changed

- ``unary`` rule now accepts ``+`` alongside ``-`` and ``~``::

      unary = ( "-" | "~" | "+" ) unary | primary ;

  SQLite documents ``+`` as a valid no-op unary prefix.  Regenerated
  ``_grammar.py`` cache via ``grammar-tools``.

## [0.41.0] - 2026-05-23

### Changed

- ``insert_body`` now accepts the ``DEFAULT VALUES`` shorthand::

      insert_body = "VALUES" row_value { "," row_value }
                  | "DEFAULT" "VALUES"
                  | query_stmt ;

  Required for SQLite's ``INSERT INTO t DEFAULT VALUES`` form, which
  inserts a single row of column defaults.  Regenerated
  ``_grammar.py`` cache via ``grammar-tools``.

## [0.40.0] - 2026-05-23

### Changed

- ``returning_clause`` now accepts ``*`` via a new ``returning_item``
  alternative::

      returning_clause = "RETURNING" returning_item { "," returning_item } ;
      returning_item   = "*" | expr ;

  Required for SQLite's ``RETURNING *`` shorthand.  Regenerated
  ``_grammar.py``.

## [0.39.0] - 2026-05-23

### Added

- ``col_constraint`` extended to accept optional ``AUTOINCREMENT``
  after ``PRIMARY KEY``::

      col_constraint = ("PRIMARY" "KEY" ["AUTOINCREMENT"]) | ... ;

  Required for SQLite-style ``id INTEGER PRIMARY KEY AUTOINCREMENT``
  column declarations.  Regenerated ``_grammar.py`` and ``_tokens.py``.

## [0.38.0] - 2026-05-23

### Added

- ``table_ref`` extended with an optional trailing ``index_hint``:
  ```
  index_hint = "INDEXED" "BY" NAME | "NOT" "INDEXED" ;
  ```
  Two SQLite-only query hints — ``INDEXED BY <name>`` (force the
  named index) and ``NOT INDEXED`` (disable index substitution).
- Regenerated ``_grammar.py`` and ``_tokens.py`` to include the new
  rule and the ``INDEXED`` keyword.

## [0.37.0] - 2026-05-23

### Added

- ``alter_table_stmt`` extended to support all four SQLite forms,
  not just ``ADD COLUMN``:

      alter_table_stmt = "ALTER" "TABLE" NAME (
            "ADD" [ "COLUMN" ] col_def
          | "RENAME" "TO" NAME
          | "RENAME" [ "COLUMN" ] NAME "TO" NAME
          | "DROP" [ "COLUMN" ] NAME
      ) ;

  The ``COLUMN`` keyword is optional everywhere (matches SQLite).
- Regenerated grammar tables.

## [0.36.0] - 2026-05-23

### Added

- ``col_constraint`` now accepts ``COLLATE name`` as one of its
  alternatives, matching SQLite's column-constraint grammar:

      col_constraint = ( "NOT" "NULL" ) | "NULL" | ( "PRIMARY" "KEY" )
                     | "UNIQUE" | ( "DEFAULT" primary )
                     | ( "CHECK" "(" expr ")" )
                     | ( "COLLATE" NAME )                            ← new
                     | ( "REFERENCES" NAME [ "(" NAME ")" ] ) ;

  This lets users declare a column's default comparison-collation at
  CREATE TABLE time: ``CREATE TABLE users(email TEXT COLLATE NOCASE)``.
- Regenerated ``_grammar.py`` to embed the new alternative.

## [0.35.0] - 2026-05-23

### Added

- New ``collated`` rule between ``bitwise`` and ``comparison`` so the
  comparison-level operators (``=``, ``<``, ``BETWEEN``, ``LIKE``,
  ``IS DISTINCT FROM``, …) accept an optional ``COLLATE name``
  postfix on either side:

      collated   = bitwise [ "COLLATE" NAME ] ;
      comparison = collated [ cmp_op collated | "BETWEEN" collated
                              "AND" collated | … ] ;

  This matches SQLite's operator precedence: ``x * y COLLATE z`` is
  ``(x * y) COLLATE z`` (multiplicative binds tighter than COLLATE),
  and ``x COLLATE y = z`` is ``(x COLLATE y) = z`` (COLLATE binds
  tighter than comparison).
- Regenerated ``_grammar.py`` to embed the new rule.

## [0.34.0] - 2026-05-22

### Added

- ``ORDER BY`` items accept an optional ``COLLATE name`` clause between
  the expression and ``ASC`` / ``DESC``:

      order_item = expr [ "COLLATE" NAME ] [ "ASC" | "DESC" ]
                   [ "NULLS" NAME ] ;

  The collation name is verbatim (any NAME token); validation happens
  in downstream layers.  Regenerated ``_grammar.py`` to embed the new
  production.

## [0.33.0] - 2026-05-22

### Added

- New `values_stmt` production handling SQLite's ``VALUES (a,b),(c,d),…``
  expression:

      values_stmt = "VALUES" row_value { "," row_value } ;

  `query_stmt` now accepts `values_stmt` as an alternative to
  `select_stmt`, and `set_op_clause` accepts it as a right operand —
  so VALUES works anywhere a SELECT does (top-level, derived table,
  CTE body, set-op operand).
- Regenerated `_grammar.py` to embed the new production.

## [0.32.0] - 2026-05-21

### Added

- Regenerated `_tokens.py` to include the `HEX_INT` token from sql-lexer
  0.24.  No grammar-rule changes were needed: `HEX_INT` aliases to
  `NUMBER`, so every existing `NUMBER` reference in the .grammar file
  (LIMIT/OFFSET, frame offsets, primary expressions, etc.) accepts
  `0x1F` and `0XDEADBEEF` literals transparently.

## [0.31.0] - 2026-05-21

### Added

- New `bitwise` precedence layer between `additive` and `comparison`:

      bitwise = additive { ( "&" | "|" | "<<" | ">>" ) additive } ;

  This matches SQLite's grammar where all four binary bitwise operators
  sit at one precedence level, looser than arithmetic but tighter than
  any comparison.  `comparison` now consumes `bitwise` instead of
  `additive` so expressions like `5 & 3 = 1` parse as `(5 & 3) = 1`.
- `unary` now accepts the bitwise-NOT prefix `~`:

      unary = ( "-" | "~" ) unary | primary ;

  `~` binds at the same precedence as unary `-`, right-associatively, so
  `-~5` parses as `-(~5)` = 6 and `~~5` parses as `~(~5)` = 5.
- Regenerated `_grammar.py` and `_tokens.py` to include the new rule
  and the `SHIFT_LEFT`/`SHIFT_RIGHT`/`BIT_AND_OP`/`BIT_OR_OP`/`BIT_NOT_OP`
  tokens.

## [0.30.0] - 2026-05-21

### Changed

- **Derived-table alias is now optional** in ``table_ref``.  The
  grammar rule changes from ``"(" query_stmt ")" [ "AS" ] NAME`` to
  ``"(" query_stmt ")" [ [ "AS" ] NAME ]`` so SQLite-style bare
  derived tables such as ``SELECT * FROM (SELECT 1 AS x)`` parse
  successfully.  Regenerated ``_grammar.py``.

## [0.29.0] - 2026-05-19

### Added

- **``ORDER BY ... NULLS FIRST | NULLS LAST``** (SQLite 3.30+).  The
  ``order_item`` rule in ``sql.grammar`` now accepts an optional
  null-placement clause::

      order_item = expr [ "ASC" | "DESC" ] [ "NULLS" NAME ] ;

  The NAME must be ``FIRST`` or ``LAST`` (case-insensitive); the
  adapter raises ``ProgrammingError`` for anything else.  Keeping
  FIRST/LAST as plain identifiers preserves their use as common
  column names (``first_name``, ``last``).

  The planner already supported per-key NULL placement via
  ``SortKey.nulls_first``; this PR just connects the grammar to it.

## [0.28.0] - 2026-05-19

### Fixed

- **Derived table aliases now accept implicit ``AS``**.  The grammar
  previously required ``"AS" NAME`` after a parenthesised subquery in a
  FROM clause::

      table_ref = "(" query_stmt ")" "AS" NAME | ... ;   (old)

  Standard SQL and SQLite accept both ``(query) AS alias`` and
  ``(query) alias`` (with AS omitted).  The rule is now::

      table_ref = "(" query_stmt ")" [ "AS" ] NAME | ... ;

  Three new parser tests in ``TestDerivedTableImplicitAS`` lock both
  forms (and a multi-source cross-join variant) as accepted.

## [0.27.0] - 2026-05-19

### Added

- **CTE ``MATERIALIZED`` / ``NOT MATERIALIZED`` hint** (`sql.grammar`,
  `_grammar.py`, `_tokens.py`) — the ``cte_def`` rule now accepts an
  optional ``[ NOT ] MATERIALIZED`` between ``AS`` and the opening
  parenthesis::

      cte_def = NAME [ "(" NAME { "," NAME } ")" ] "AS"
                [ [ "NOT" ] "MATERIALIZED" ]
                "(" query_stmt ")" ;

  This is SQLite 3.35+ syntax: the hint tells SQLite's planner whether
  to materialise the CTE result set or inline it.  Mini-sqlite has no
  cost-based optimizer, so the keywords are parsed and silently
  ignored at the adapter level — applications using them for
  portability still parse and execute correctly.  Five new parser
  tests in ``TestCteMaterializedHint`` lock the grammar acceptance.

## [0.26.0] - 2026-05-19

### Added

- **SQLite conditional-upsert WHERE clause** (`sql.grammar`,
  `_grammar.py`) — the `DO UPDATE` branch of `upsert_clause` now accepts
  an optional trailing `WHERE expr`:

      upsert_clause = "ON" "CONFLICT"
                      [ "(" NAME { "," NAME } ")" ]
                      ( "DO" "NOTHING"
                      | "DO" "UPDATE" "SET" upsert_assignment { "," upsert_assignment }
                        [ where_clause ] ) ;

  This is SQLite 3.24+ syntax for *conditional upserts* — the SET
  assignments fire only when the predicate is true.  Example::

      INSERT INTO t VALUES (1, 99)
      ON CONFLICT(id) DO UPDATE SET v = excluded.v WHERE excluded.v > v

## [0.25.0] - 2026-05-18

### Added

- **`STRICT` and `WITHOUT ROWID` table options** (`sql.grammar`,
  `_grammar.py`) — `create_table_stmt` now accepts an optional
  comma-separated list of table options after the closing `)`:

      create_table_stmt = ... ")" [ table_options ] ;
      table_options     = table_option { "," table_option } ;
      table_option      = "STRICT" | "WITHOUT" NAME ;

  The `WITHOUT NAME` form intentionally uses NAME (not a `ROWID` keyword)
  so that `rowid` remains a valid column name elsewhere in SQL.  Both
  options are syntactically accepted and semantically ignored by
  mini-sqlite.

## [0.24.0] - 2026-05-18

### Added

- **`attach_stmt` / `detach_stmt`** (`sql.grammar`, `_grammar.py`) — two
  new top-level statement rules:

      attach_stmt = "ATTACH" [ "DATABASE" ] expr "AS" NAME ;
      detach_stmt = "DETACH" [ "DATABASE" ] NAME ;

  Added as alternatives in the top-level `statement` rule.  The
  `DATABASE` keyword is optional (SQLite accepts both `ATTACH DATABASE …`
  and `ATTACH …`).

## [0.23.0] - 2026-05-18

### Added

- **Indexed expressions in `CREATE INDEX`** (`sql.grammar`, `_grammar.py`)
  — the `index_col` rule now accepts a full `expr` (not just a bare NAME),
  plus an optional `COLLATE name` clause and the existing optional
  `ASC`/`DESC`.  This enables SQLite-compatible indexed-expression syntax:

      CREATE INDEX idx ON t(LOWER(name))
      CREATE INDEX idx ON t(name COLLATE NOCASE)
      CREATE INDEX idx ON t(LOWER(name), id ASC)

  The grammar also accepts an optional trailing `WHERE` clause on
  `CREATE INDEX` for the SQLite "partial index" syntax.  The adapter
  silently ignores the predicate; partial-index lookup is not implemented.

## [0.22.0] - 2026-05-17

### Added

- **JSON path-shortcut operators in `additive`** (`sql.grammar`,
  `_grammar.py`) — the `additive` rule now accepts `JSON_ARROW` (`->`)
  and `JSON_ARROW_TEXT` (`->>`) alongside `+`, `-`, and `||`.  Both are
  left-associative and have the same precedence as the existing additive
  operators.

## [0.21.0] - 2026-05-17

### Added

- **`LIKE … ESCAPE 'c'`** (`sql.grammar`, `_grammar.py`) — the `comparison`
  rule now accepts an optional `"ESCAPE" additive` after `LIKE pattern` and
  `NOT LIKE pattern`.  When present the third additive is the escape
  character (must be a single-character string literal).

## [0.20.0] - 2026-05-15

### Changed

- **`col_type` grammar rule** (`sql.grammar`, `_grammar.py`) — Column types in
  `CREATE TABLE` now support optional length/precision parameters:

      col_type = NAME [ "(" NUMBER { "," NUMBER } ")" ] ;

  This enables `VARCHAR(30)`, `DECIMAL(10, 2)`, `CHAR(8)` and similar
  parameterised type specifications that are common in real-world SQL schemas.
  The adapter (`mini_sqlite/adapter.py`) was updated to extract the type name
  from the new `col_type` child node.

- **`IN ()` empty list** (`sql.grammar`, `_grammar.py`) — The grammar now accepts
  `IN ()` and `NOT IN ()` with an empty value list.  Previously the parser
  required at least one value, causing `Parse error` on valid SQL.  The adapter
  returns an `In`/`NotIn` node with an empty `values=()` tuple.

- **Comma-separated `FROM` clause** (`sql.grammar`, `_grammar.py`) — `FROM t1, t2`
  implicit cross-join syntax is now accepted.  Each comma-joined table is treated
  as a `CROSS JOIN` internally.

- **`CREATE INDEX` column list with `ASC`/`DESC`** (`sql.grammar`, `_grammar.py`) —
  The `CREATE INDEX` column list now accepts optional `ASC` or `DESC` on each
  column name (`index_col` rule).  Previously only bare column names were
  accepted, causing a parse error on `CREATE INDEX idx ON t(col DESC)`.

## [0.19.0] - 2026-05-15

### Changed

- **`cte_def` grammar rule** (`sql.grammar`, `_grammar.py`) — Extended `cte_def`
  with an optional column-alias list between the CTE name and `AS`:

      cte_def = NAME [ "(" NAME { "," NAME } ")" ] "AS" "(" query_stmt ")" ;

  This allows CTE definitions to declare explicit column names, which is required
  by many real-world SQL queries and is standard SQL syntax:

      WITH RECURSIVE cnt(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM cnt WHERE n<5)
      SELECT n FROM cnt;

  Queries without a column list continue to work unchanged — the new `[ … ]`
  group is optional.

## [0.18.0] - 2026-05-14

### Added

- **`filter_clause` grammar rule** (`sql.grammar`, `_grammar.py`) — Extended the
  `function_call` rule with an optional `filter_clause` suffix:

      function_call = (NAME | "REPLACE") "(" ( STAR | "DISTINCT" value_list | [ value_list ] ) ")" [ filter_clause ] ;
      filter_clause = "FILTER" "(" "WHERE" expr ")" ;

  This allows the SQL parser to recognise the SQL:2003 / SQLite FILTER (WHERE …)
  syntax on aggregate function calls such as:

      COUNT(*) FILTER (WHERE active = 1)
      SUM(salary) FILTER (WHERE dept = 'eng')

  The `_grammar.py` file was regenerated from `sql.grammar` via the
  `grammar_tools` CLI.

## [0.17.0] - 2026-05-13

### Added

- **`IS DISTINCT FROM` / `IS NOT DISTINCT FROM` comparison syntax** (`_grammar.py`,
  `sql.grammar`) — the `comparison` grammar rule's optional trailing suffix is
  extended with two new alternatives:

      | "IS" "DISTINCT" "FROM" additive
      | "IS" "NOT" "DISTINCT" "FROM" additive

  These appear after the existing `IS NULL` / `IS NOT NULL` alternatives so that
  the parser greedily matches the longer keyword sequence first.

  The adapter's `_comparison` helper detects the `DISTINCT` keyword child to
  distinguish these new operators from the existing `IS NULL` / `IS NOT NULL` forms
  and emits the corresponding `BinaryExpr(op=BinaryOp.IS_DISTINCT_FROM, …)` or
  `BinaryExpr(op=BinaryOp.IS_NOT_DISTINCT_FROM, …)` plan node.

## [0.16.0] - 2026-05-13

### Fixed

- **`''` (doubled-quote) escape in string literals** (`_tokens.py`, `sql.tokens`) —
  the `STRING_SQ` token regex was `'([^'\\]|\\.)*'` which could not match a string
  containing `''` (two consecutive quotes, the ANSI SQL escape for a literal
  single-quote).  The updated regex is `'(''|[^'\\]|\\.)*'` — the `''` alternative
  is tried first so a pair of apostrophes is consumed as a unit rather than
  terminating the string early.  This allows `SELECT 'O''Brien'` to produce
  `O'Brien` rather than a parse error.

- **`REPLACE()` as a function name** (`_grammar.py`, `sql.grammar`) — the
  `function_call` grammar rule previously required the function name to be a `NAME`
  token.  Because `REPLACE` is a keyword (used for `REPLACE INTO` DML), it was
  tokenised as `KEYWORD` and rejected.  The rule now accepts either `NAME` or the
  literal keyword `REPLACE`, enabling `REPLACE(str, from, to)` scalar-function
  calls alongside the existing `REPLACE INTO` DML syntax.

- **`COUNT(DISTINCT col)` parsing** (`_grammar.py`, `sql.grammar`) — the
  `function_call` rule's argument alternation is extended from:

      STAR | Optional(value_list)

  to:

      STAR | Sequence([Literal('DISTINCT'), value_list]) | Optional(value_list)

  This lets `COUNT(DISTINCT col)`, `SUM(DISTINCT col)`, etc. parse without
  ambiguity.  The `DISTINCT` keyword is detected in `_function_call` (adapter) and
  forwarded as `distinct=True` on the resulting `AggregateExpr`.

## [0.15.0] - 2026-05-04

### Added

- **`conflict_clause` grammar rule** (`sql.grammar`) — new production
  `conflict_clause = "OR" ( "REPLACE" | "IGNORE" | "ABORT" | "FAIL" | "ROLLBACK" )`
  captures SQLite's conflict-resolution clause.  Matches the `OR` keyword
  followed by any of the five conflict actions.

- **`insert_stmt` extended with optional `conflict_clause`** — INSERT now
  parses as `"INSERT" [ conflict_clause ] "INTO" NAME …`, enabling
  `INSERT OR REPLACE INTO …`, `INSERT OR IGNORE INTO …`, etc.

- **`replace_stmt` grammar rule** — new top-level statement rule
  `replace_stmt = "REPLACE" "INTO" NAME …` provides the `REPLACE INTO`
  shorthand (SQLite syntactic sugar for `INSERT OR REPLACE INTO`).

- **`statement` and `trigger_body_stmt` updated** — both alternations now
  include `replace_stmt` as a valid choice alongside `insert_stmt`.

- **Regenerated `_grammar.py`** — the pre-compiled parser grammar cache now
  reflects all grammar changes above.

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

