# Changelog

## 0.5.30 — Expr-level `COLLATE` in comparisons

`x = y COLLATE NOCASE`, `a < b COLLATE RTRIM`, `WHERE name = 'foo' COLLATE NOCASE`
now parse and run with SQLite's semantics: the collation applies to the whole
comparison (equality and ordering), NOCASE folds ASCII case, RTRIM ignores
trailing spaces, and a numeric operand is unaffected (`5 = '5' COLLATE NOCASE` is
0). It is lowered entirely in the planner (sql-parser 0.1.14 grammar; sql-planner
0.2.13) onto a new internal `__collate` builtin (sql-vm 0.4.16) that canonicalises
each operand — no new comparison opcode, mirroring `GLOB → glob()`. The trick:
NOCASE/RTRIM are canonicalising transforms, so `x <op> y COLLATE C` equals
`canon_C(x) <op> canon_C(y)` under byte comparison, exactly. Five differential-
oracle cases (=, <, WHERE, RTRIM, numeric-operand) diff against real bundled
SQLite. (COLLATE on the LEFT operand — `col COLLATE NOCASE = 'x'` — is a follow-up.)

## 0.5.29 — Bitwise operators `& | ~ << >>`

`SELECT 5 & 3`, `5 | 2`, `~0`, `1 << 4`, `256 >> 2` now parse and run with
SQLite's exact semantics: operands are coerced to integer (integer affinity —
reals truncate toward zero, `2.9 & 1` → 0; text prefix-parses), NULL propagates,
and the four binary operators share one left-associative precedence level between
additive and comparison (`5 | 3 & 2` = `(5|3)&2` = 2; `3 + 1 << 2` = 16). Shifts
follow SQLite's rules precisely — a negative count flips direction (`1 << -1` =
`1 >> 1` = 0), a count ≥ 64 saturates (left → 0; right → 0 for non-negative, −1
for negative), and right shift is arithmetic (`-1 >> 1` = -1) — implemented
without Rust's shift-overflow UB. Full pipeline: lexer tokens (sql-lexer 0.1.2),
grammar `bitwise` level + `~` prefix (sql-parser 0.1.13), `BitAnd`/`BitOr`/
`ShiftLeft`/`ShiftRight`/`BitNot` (sql-planner 0.2.12, sql-codegen 0.6.5), and VM
execution with `sql_shift` (sql-vm 0.4.15). Seven differential-oracle cases diff
each operator, precedence, real truncation, NULL, shift edges, and a table column
against real bundled SQLite.

## 0.5.28 — Simple (operand) `CASE x WHEN v THEN …`

`SELECT CASE x WHEN 1 THEN 'a' WHEN 2 THEN 'b' ELSE 'c' END` now parses and runs
with SQLite's semantics: the operand is compared to each `WHEN` value for
equality, the first match's result is returned (ELSE, or NULL if no ELSE and no
match). A NULL operand matches nothing (`x = NULL` is never true) and falls
through to ELSE. It is lowered entirely in the planner (sql-parser 0.1.12 adds
the optional operand to the CASE grammar; sql-planner 0.2.11 desugars each
`WHEN v THEN r` into a `(x = v, r)` branch of the searched-CASE node added in
0.5.25) — so **no codegen or VM opcode**; it reuses the searched `CASE`
machinery. Four differential-oracle cases (with/without ELSE, text operand +
first-match, NULL-operand → ELSE) diff against real bundled SQLite.

## 0.5.27 — `COLLATE` in `ORDER BY` (NOCASE / RTRIM / BINARY)

`ORDER BY col COLLATE name` now sorts through a collating sequence, matching
SQLite: `NOCASE` compares ASCII case-insensitively (`'Apple'` = `'apple'`),
`RTRIM` ignores trailing spaces (`'a  '` = `'a'`), and `BINARY` (the default)
keeps raw byte order (uppercase before lowercase). The `COLLATE` name parses
between the sort expression and `ASC`/`DESC` (sql-parser 0.1.11), is validated
in the planner into a new `SortKey.collation` field (sql-planner 0.2.10),
threaded through `CompiledSortKey` (sql-codegen 0.6.4) and constant folding
(sql-optimizer 0.1.4), and applied by the VM sort comparator to text values
only (sql-vm 0.4.14). Equal keys keep insertion order (the sort is stable),
matching SQLite. Four differential-oracle cases diff NOCASE (asc + desc), RTRIM,
and BINARY against real bundled SQLite. Unknown collations are a planning error.

## 0.5.26 — `IS` / `IS NOT` null-safe (in)equality

`SELECT a IS b` / `a IS NOT b` now parse and run as SQLite's null-safe
comparison: `a IS b` is 1 when both operands are equal OR both NULL, 0 when
exactly one is NULL (unlike `=`, which yields NULL if either side is NULL);
`a IS NOT b` is the negation. It is lowered entirely in the planner onto the
CASE node (added in 0.5.25) — `CASE WHEN a IS NULL AND b IS NULL THEN 1 WHEN a
IS NULL OR b IS NULL THEN 0 ELSE a=b END` — so grammar (sql-parser 0.1.10) +
planner (sql-planner 0.2.9) are the only changes; **no codegen or VM opcode**.
`IS NULL` / `IS NOT NULL` still work (matched first). Two differential-oracle
cases (`is_null_safe_equality`, `is_operator_in_where`) diff against real
bundled SQLite, including `IS` as a WHERE predicate. (`IS [NOT] DISTINCT FROM`
— the standard-SQL spelling — is a separate follow-up.)

## 0.5.25 — Searched `CASE WHEN … THEN … [ELSE …] END`

`SELECT CASE WHEN cond THEN val … [ELSE val] END` now parses and runs with
SQLite's exact semantics: branches are evaluated top-to-bottom, the first
truthy `WHEN` yields its `THEN`; no match with an `ELSE` yields the `ELSE`,
otherwise `NULL`; a `NULL` condition is not truthy (its branch is skipped); and
evaluation short-circuits (later branches' values are not computed once one
matches). Spans grammar (sql-parser 0.1.9) → `SqlExpr::Case` (sql-planner
0.2.8) → a jump-chain in codegen (sql-codegen 0.6.3, reusing the existing
`JumpIfTrue`/`Jump`/`Label` opcodes — no VM change), with the optimizer (0.1.3)
folding through the node. Three differential-oracle cases (`case_first_match_and_else`,
`case_no_else_is_null_and_null_cond_skipped`, `case_in_where_and_arithmetic`)
diff against real bundled SQLite, including CASE nested in a WHERE predicate and
in arithmetic. (The *simple* form `CASE x WHEN v THEN …` — equality against a
base expression — is a separate follow-up slice.)

## 0.5.24 — `NULLS FIRST` / `NULLS LAST` in `ORDER BY`

`SELECT … ORDER BY a NULLS FIRST` / `NULLS LAST` now parse and run. mini-sqlite
already reproduced SQLite's *default* null ordering (NULLs first for ASC, last
for DESC) for free — this adds the explicit clause, whose value matters most for
the OVERRIDE cases (`ASC NULLS LAST`, `DESC NULLS FIRST`). Spans grammar
(sql-parser 0.1.8) → `SortKey.nulls_first` (sql-planner 0.2.7) →
`CompiledSortKey` (sql-codegen 0.6.2) → the VM sort comparator (sql-vm 0.4.13),
with the optimizer (0.1.2) carrying the field. FIRST/LAST stay non-reserved
(validated in the planner). Three differential-oracle cases
(`order_by_nulls_last`, `order_by_desc_nulls_first`,
`order_by_nulls_first_default`) diff row order against real bundled SQLite.
COLLATE in ORDER BY (a comparator transform) remains a separate follow-up.

## 0.5.23 — `CAST(expr AS type)` — INTEGER / REAL / TEXT

`SELECT CAST(x AS INTEGER)` and friends now parse and run, following SQLite's
documented cast rules: text yields its leading integer prefix (`'12abc'`→12,
`'3.9'`→3), reals truncate toward zero (`-4.9`→-4), text→REAL reads the leading
real prefix incl. exponent (`'1e3'`→1000.0), and integers render to their
decimal string for TEXT. The declared type name resolves through SQLite's
substring **affinity** rule, so synonyms like `INT`, `VARCHAR`, and `FLOAT`
work. This spans all layers: grammar (sql-parser 0.1.7) → `SqlExpr::Cast`
(sql-planner 0.2.6) → `Instruction::Cast` (sql-codegen 0.6.1) → `apply_cast`
(sql-vm 0.4.12), with the optimizer (0.1.1) recursing through the new node.
Three differential-oracle cases (`cast_to_integer`, `cast_to_real`,
`cast_to_text_and_synonyms`) diff against real bundled SQLite. `BLOB` and
`NUMERIC` target types, and `real→TEXT` formatting (the approximate-`dtoa`
rabbit hole), remain follow-ups.

## 0.5.22 — `GLOB` / `NOT GLOB` infix operators

`SELECT … WHERE s GLOB 'x*'` now parses and runs — case-sensitive Unix-glob
matching (`*` any run, `?` one char, `[…]` classes). SQLite defines the
operator `X GLOB Y` as the function `glob(Y, X)`, and mini-sqlite lowers it
exactly that way: the grammar accepts `GLOB`/`NOT GLOB` as comparison forms
(sql-parser 0.1.6) and the planner rewrites them onto the existing `glob`
builtin (sql-planner 0.2.5, args swapped; `NOT GLOB` = `NOT glob(...)`). No new
codegen or VM opcode was needed — it reuses the scalar-function path. Two new
differential-oracle cases (`glob_operator`, `not_glob_and_question`) diff
against real bundled SQLite, including case-sensitivity and the `?`
single-char wildcard. (The `glob()` function form has been available since
0.4.x; this adds the infix operator spelling.)

## 0.5.21 — `LIMIT off, count` MySQL shorthand

`SELECT … LIMIT 1, 2` now parses and runs, returning the same rows as
`LIMIT 2 OFFSET 1` — SQLite accepts this MySQL-compatibility spelling. The
catch is the flipped argument order: in the comma form the FIRST number is the
offset and the SECOND is the count. Grammar accepts `, NUMBER` as a `LIMIT`
tail (sql-parser 0.1.5); the planner detects the comma and swaps
offset/count (sql-planner 0.2.4). Codegen and the VM already consume the
`Limit { count, offset }` plan unchanged, so no lower-layer work was needed.
Two new differential-oracle cases (`limit_comma_offset_count`,
`limit_comma_matches_offset`) diff the row window against real bundled SQLite.

## 0.5.20 — Table alias without the `AS` keyword

`FROM users u` now aliases the table exactly like `FROM users AS u` — SQLite
(and standard SQL) accept both spellings. The generated sql-parser grammar
required `AS`; making it optional (sql-parser 0.1.4) plus teaching the planner's
`extract_table_ref` to read a bare trailing `NAME` token (sql-planner 0.2.3)
fixes it, including qualified references through the bare alias (`u.id`) and
bare aliases on both sides of a `JOIN`. Two new differential-oracle cases
(`table_alias_without_as`, `join_bare_table_alias`) diff against real bundled
SQLite. This is the sibling of 0.5.19 (column aliases) — the same idea in the
`table_ref` grammar slot. (Comma joins `FROM a, b`, `USING`, and `NATURAL JOIN`
still need new planner work.)

## 0.5.19 — Column alias without the `AS` keyword

`SELECT a col1` now parses and names the output column `col1`, exactly like
`SELECT a AS col1` — SQLite (and standard SQL) accept both spellings. The
generated sql-parser grammar required `AS`; making it optional (sql-parser
0.1.3) plus teaching the planner's alias extractor to read a bare trailing
`NAME` token (sql-planner 0.2.2) fixes it. Two new differential-oracle cases
(`column_alias_without_as`, `bare_alias_matches_as`) diff the resulting
**column names** against real bundled SQLite. (Table aliases without `AS`,
`FROM t u`, are the same idea in a different grammar slot and remain a
follow-up.)

## 0.5.18 — Conditionless joins (Cartesian product)

`SELECT … FROM a CROSS JOIN b` and `SELECT … FROM a JOIN b` (no `ON`) now parse
and run as a Cartesian (cross) product. The generated sql-parser grammar required
an `ON` after every join; making `ON expr` optional (sql-parser 0.1.2) fixes it,
and the planner/codegen already produce a cross product for a conditionless join.
Two new differential-oracle cases (`cross_product_no_on`, `join_no_on_is_cross`)
diff against real bundled SQLite. (Comma joins `FROM a, b`, `USING`, and `NATURAL
JOIN` remain unsupported — those need new planner work, not just a grammar tweak.)

## 0.5.17 — Bare `JOIN` (INNER by default)

`SELECT … FROM a JOIN b ON …` now parses and runs — a bare `JOIN` (no
`INNER`/`LEFT`/… keyword) is an INNER join. The generated sql-parser grammar
required an explicit `join_type` before `JOIN`; making it optional (sql-parser
0.1.1) fixes it, and the planner already defaulted a missing type to INNER. A new
differential-oracle case (`bare_join`) confirms it produces the same rows as
`INNER JOIN` against real bundled SQLite.

## 0.5.16 — Parse `''` escaped single quotes in string literals

A fundamental correctness fix: a doubled single quote (`''`) inside a SQL string
literal is the escape for one literal quote, so `'it''s'` is the string `it's`
and `'O''Brien'` is `O'Brien`. mini-sqlite failed to parse these — the generated
sql-lexer token grammar had a stale backslash-escape regex, so `'it''s'`
tokenized as two adjacent strings. Fixed in sql-lexer 0.1.1 (SQL `''` regex) and
sql-planner 0.2.1 (`''` → `'` when building the string value). A new
differential-oracle case (`escaped_quote_literal`) covers escaped quotes in both
the SELECT list and INSERT'd row values against real bundled SQLite. This also
unblocks testing PRINTF's `%q` via ordinary string literals.

## 0.5.15 — PRINTF / FORMAT (string formatting)

Grows the SQL scalar surface (sql-vm 0.4.11): `PRINTF(format, …)` and its alias
`FORMAT(…)` do C-style string formatting — `%d`/`%i`, `%s` (with precision),
`%x`/`%X`, `%o`, `%c`, `%q`, `%%`, with `-`/`0`/`+`/space flags and a field
width. Missing args default to 0/"", extra args are ignored, NULL format → NULL,
and the width/output are capped against DoS. Float conversions are declined. Two
new differential-oracle cases (`printf`, `printf_edges`) diff against real
bundled SQLite. (Testing `%q` via a `''` string literal surfaced a separate,
pre-existing lexer gap — mini can't parse doubled-quote escapes — so the oracle
gets its quote from `CHAR(39)` instead; the lexer fix is tracked separately.)

## 0.5.14 — GLOB() function

Grows the SQL scalar surface (sql-vm 0.4.10): `GLOB(pattern, subject)` — the
function form of the `GLOB` operator — does a case-sensitive wildcard match
(`*`, `?`, `[...]` classes) returning 1/0, NULL if either argument is NULL. The
matcher is `O(text × pattern)` (no ReDoS-style blow-up). A new differential-oracle
case (`glob_function`) diffs against real bundled SQLite. (The infix `x GLOB y`
operator is grammar-blocked and remains unsupported — see
project_minisqlite_conformance_probe_map.)

## 0.5.13 — LIKELY / UNLIKELY / LIKELIHOOD (planner hints)

Grows the SQL scalar surface (sql-vm 0.4.9) with SQLite's query-planner hint
functions: `LIKELY(x)`, `UNLIKELY(x)`, and `LIKELIHOOD(x, p)`. They're the
identity function on their first argument (any type, including NULL) — the hint
only nudges the optimizer. `LIKELIHOOD`'s probability `p` must be a number in
`[0,1]`. A new differential-oracle case (`likely_family`) diffs against real
bundled SQLite.

## 0.5.12 — OCTET_LENGTH (byte length)

Grows the SQL scalar surface (sql-vm 0.4.8): `OCTET_LENGTH(x)` returns the byte
count of a value, where `LENGTH` returns the character count — `OCTET_LENGTH('héllo')`
is 6 but `LENGTH('héllo')` is 5. Text is measured as UTF-8 bytes, a blob by its
raw bytes, an integer by its decimal digits; NULL → NULL; floats declined. A new
differential-oracle case (`octet_length`) diffs `OCTET_LENGTH(s)` and `LENGTH(s)`
across multibyte, empty, and NULL rows against real bundled SQLite.

## 0.5.11 — Fix HEX(NULL) to return an empty string

Stream A correctness fix (sql-vm 0.4.7): `HEX(NULL)` returned SQL NULL but real
SQLite returns the empty string `''` — it casts the argument to a blob first, so
NULL → empty blob → `''` (a text value). This was the latent divergence flagged
by the UNHEX work (0.5.10). A new differential-oracle case (`hex_of_null`) diffs
`HEX(s)` and `TYPEOF(HEX(s))` for both a text and a NULL row against real bundled
SQLite.

## 0.5.10 — UNHEX (decode hex to blob)

Grows the SQL scalar surface (sql-vm 0.4.6): `UNHEX(x)` / `UNHEX(x, ignore)`
decodes hexadecimal digit pairs into a blob — the inverse of `HEX`. Odd length
or non-hex characters → NULL; the optional ignore-set is honoured only at byte
boundaries (`unhex('41.42','.')` → `x'4142'`, `unhex('4-1-4-2','-')` → NULL).
Two new differential-oracle cases (`unhex`, `unhex_ignore_set`) diff against real
bundled SQLite.

Note: adding UNHEX surfaced a separate, pre-existing divergence — `HEX(NULL)`
returns NULL here but real SQLite returns an empty string. That is left for its
own fix; the UNHEX oracle cases compare blobs directly to avoid it.

## 0.5.9 — Fix ROUND with a negative digit count

Stream A correctness fix (sql-vm 0.4.5): `ROUND(x, n)` with a negative `n` now
matches SQLite, which treats it as zero digits rather than rounding to
tens/hundreds — `ROUND(2.567, -1)` returned `0.0` but SQLite gives `3.0`
(= `ROUND(2.567, 0)`), and `ROUND(12.5, -1)` = `13.0`. A new differential-oracle
case (`round_negative_digits`) diffs against real bundled SQLite.

## 0.5.8 — CONCAT / CONCAT_WS / SUBSTRING string functions

Grows the SQL scalar surface (sql-vm 0.4.4): `CONCAT(x, …)` joins all arguments
(NULL → empty string), `CONCAT_WS(sep, …)` joins with a separator (skipping NULL
values, NULL separator → NULL), and `SUBSTRING` is accepted as a spelling of
`SUBSTR`. Previously all three errored as unknown built-ins. Five new
differential-oracle cases (`concat`, `concat_ws`, `concat_ws_null_sep`,
`substring_alias`) diff against real bundled SQLite. Integer/boolean arguments
coerce to text; Float/Blob are declined (matching the HEX/QUOTE convention — a
SQLite-exact float-to-text formatter is a separate future step).

## 0.5.7 — Read `WITHOUT ROWID` tables from real .sqlite files

Querying a `WITHOUT ROWID` table in a real `.sqlite` file now works through the
whole pipeline; previously it failed with `unexpected b-tree page type` because
such tables live in an *index* b-tree, not a table b-tree. Delivered by
`sqlite-file` 0.7.0 (`read_without_rowid_table` over `walk_index`) and
storage-sqlite 0.4.0 (`WITHOUT ROWID` detection + read path). A new
`file_backed` differential test builds real `WITHOUT ROWID` tables (scalar /
TEXT / composite primary keys, and an 800-row table spanning interior index
pages) and diffs mini-sqlite against real bundled SQLite.

## 0.5.6 — Two-argument TRIM/LTRIM/RTRIM (character-set trimming)

Grows the SQL scalar surface: `TRIM(x, y)`, `LTRIM(x, y)`, and `RTRIM(x, y)`
now strip a caller-supplied *set of characters* rather than only whitespace —
`TRIM('xxhixx', 'x')` → `'hi'`, `TRIM('abcHIcba', 'abc')` → `'HI'`. Previously
these errored (`TRIM expects 1 arg, got 2`). Trimming is Unicode-character-aware,
an empty set is a no-op, NULL in either argument propagates, and numeric
arguments coerce to text — all matching real SQLite (sql-vm 0.4.3). Three new
differential-oracle cases (`trim_charset`, `trim_charset_multi`,
`trim_charset_null_and_edge`) diff against real bundled SQLite; the
single-argument whitespace forms are unchanged.

## 0.5.5 — Multi-argument MAX/MIN are the scalar functions

Stream A correctness fix: `SELECT MAX(3, 9, 5)` returned `3` (the first argument)
instead of `9`, because the planner treated *every* `MIN`/`MAX` as the aggregate.
Two-or-more-argument `MIN`/`MAX` are the SCALAR largest/smallest functions and
are now dispatched as such (sql-planner 0.2.0 gates on arity; sql-vm 0.4.2 adds
the scalar builtin); the single-argument aggregate forms are unchanged. Three
new differential-oracle cases (`scalar_max_min`, `scalar_max_null`, plus an
`agg_max_min_still_work` regression guard) diff against real bundled SQLite.

## 0.5.4 — IIF(x, y, z) — the function form of CASE

Stream B, corpus growth. `IIF(x, y, z)` now works — SQLite's function-form
conditional (`CASE WHEN x THEN y ELSE z END`), a pure-`sql-vm` addition (0.4.1)
that covers much of `CASE`'s utility while full `CASE`/`CAST`/window syntax
remains blocked on a stale generated grammar. One new differential-oracle case
(`iif`) diffs mini-sqlite against real bundled SQLite. No mini-sqlite `src/`
change.

## 0.5.3 — More scalar functions: SIGN / UNICODE / CHAR / ZEROBLOB / QUOTE

Stream B, corpus-growth phase. Five more common scalar functions now work:
`SIGN`, `UNICODE`, `CHAR`, `ZEROBLOB`, `QUOTE`. They already parsed as function
calls but hit the engine's `unknown built-in function` fallthrough; implemented
in `sql-vm` (0.4.0). Five new differential-oracle cases (`sign`, `unicode`,
`char_fn`, `zeroblob`, `quote`) diff mini-sqlite's results against real bundled
SQLite. No mini-sqlite `src/` change.

## 0.5.2 — Introspect a file's indexes (`list_indexes`)

Stream C: the file backend's `list_indexes` now reports a real `.sqlite` file's
indexes (name, unique, columns, auto) from the parsed catalog — see
`storage-sqlite` 0.3.0. New differential test
`tests/file_backed.rs::list_indexes_matches_real_sqlite` diffs the result
against real SQLite's `PRAGMA index_list` / `PRAGMA index_info`. No mini-sqlite
`src/` change.

## 0.5.1 — Scalar functions: IFNULL / NULLIF / TYPEOF / INSTR / HEX

Stream A, corpus-growth phase (the differential ledger is at zero, so the
metric is now the *size* of the seed corpus that matches real SQLite). Five
common scalar functions used by real-world queries now work: `IFNULL`,
`NULLIF`, `TYPEOF`, `INSTR`, `HEX`. They already parsed as function calls but
hit the engine's `unknown built-in function` fallthrough; implemented in
`sql-vm` (0.3.0). Five new differential-oracle cases (`ifnull`, `nullif`,
`typeof`, `instr`, `hex`) diff mini-sqlite's results against real bundled
SQLite. No mini-sqlite `src/` change.

## 0.5.0 — Differential conformance ledger driven to ZERO

Stream B / L3: aggregate output columns are now named the SQLite way —
`SELECT COUNT(*)` returns a column named `COUNT(*)` (likewise `SUM(n)`,
`MIN(n)`, `MAX(n)`, `AVG(n)`), not the engine-internal `agg_N`. The fix is in
`sql-codegen` (0.6.0); no mini-sqlite `src/` change.

This retires the **last five** entries of the differential-conformance
`LEDGER` in `tests/differential_oracle.rs` (`count_star`, `sum_min_max`, `avg`,
`group_by`, `having`) — **the ledger is now empty**. Every one of the harness's
seed cases now matches real bundled SQLite exactly: `INNER`/`LEFT`/`RIGHT`/`FULL`
joins, scalar functions and their result columns, and aggregate naming. The
oracle opened at ten reproduced gaps and has been closed out one PR at a time;
it now enforces full agreement on the seed corpus and stands ready to catch the
next divergence a new case surfaces.

## 0.4.3 — Query `sqlite_master` over a real file

Stream C / L4: the schema catalog `sqlite_master` (and its alias `sqlite_schema`)
is now queryable over a real `.sqlite` file — `SELECT name FROM sqlite_master
WHERE type = 'table'`, `SELECT COUNT(*) FROM sqlite_master`, the full
five-column shape, and so on. Applications (Anki included) introspect the
database this way. The `storage-sqlite` backend (0.2.0) exposes the catalog it
already parses; no mini-sqlite `src/` change. New differential test
`tests/file_backed.rs::sqlite_master_is_queryable_and_matches_real_sqlite` diffs
the results against real bundled SQLite over the same file.

## 0.4.2 — FULL OUTER JOIN matches SQLite (ledger 6 → 5)

Stream A / L2 of the full-SQLite-replacement roadmap: retire the differential
oracle's `full_join` divergence — the last *wrong-result* entry in the ledger.

- `SELECT ... FROM a FULL JOIN b ON ...` previously degraded to a cross product
  (a single forward pass can't emit the right rows that matched no left row).
  `sql-codegen` (0.5.0) now compiles FULL JOIN as **two passes**: a LEFT JOIN
  (matched pairs + left-only rows) unioned with a RIGHT anti-join (right rows
  that matched no left row, NULL-padded on the left). No new VM instructions —
  it reuses the outer-join match flag. See that crate's changelog.
- `tests/differential_oracle.rs`: removed `full_join` from the `LEDGER` (now
  **5** entries, all aggregate computed-column *naming* divergences whose rows
  already match). Added a second FULL JOIN case, `full_join_multi`, with
  duplicate join keys on both sides (many-to-many) plus rows unmatched on each
  side — asserted against real SQLite to guard the two-pass implementation
  against double-emitting matched pairs or dropping an anti-join row.
- No mini-sqlite `src/` changes — the fix lands in the shared pipeline; this
  crate's bump documents the conformance gain the oracle now enforces.

## 0.4.1 — Scalar functions match SQLite (ledger 7 → 6)

Stream B / L3 of the full-SQLite-replacement roadmap: retire the differential
oracle's `string_functions` divergence, the last *wrong-value* entry in the
ledger.

- `SELECT UPPER(name), LENGTH(name)` previously came back as `LENGTH(name),
  LENGTH(name)` with both columns named `?`. Two independent bugs, one symptom:
  - **sql-vm** (0.2.1): Phase-4 materialization collapsed each row's positional
    `(name, value)` pairs through a `HashMap` keyed by column name, so two
    same-named output columns kept only the *last* value. Now projects by
    position (the row buffer is already parallel to the locked column list).
  - **sql-codegen** (0.4.0): un-aliased function columns were labelled `?`. Now
    labelled with the reconstructed call text (`UPPER(name)`), matching SQLite.
- `tests/differential_oracle.rs`: removed `string_functions` from the `LEDGER`
  (now **6** entries — `full_join` plus five aggregate computed-column *naming*
  divergences); the case is now asserted to match real SQLite exactly.
- No mini-sqlite `src/` changes — the fixes land in the shared pipeline crates;
  this crate's bump documents the conformance gain the oracle now enforces.

## 0.4.0 — Open a real `.sqlite` file

Stream C / L4 of the full-SQLite-replacement roadmap: `connect()` can now open a
**real SQLite database file**, and the entire query pipeline (parser → planner →
optimizer → codegen → VM) runs unmodified over it.

- `connect("<path>")` reads the file's bytes and drives the engine through the
  read-only `SqliteFileBackend` (`storage-sqlite`, built on the zero-dep
  `sqlite-file` reader — no third-party SQLite at runtime). `":memory:"` is
  unchanged. A missing file or non-SQLite bytes surface as `OperationalError`;
  the previous blanket `NotSupportedError` for any non-`:memory:` name is gone.
- `ConnectionState.backend` is now `Box<dyn Backend>` so either backend plugs in;
  the connection tracks its own transaction handle (`current_transaction` is not
  a `Backend`-trait method). File-backed connections are **read-only** for now —
  `INSERT`/`UPDATE`/`CREATE` against a file return an error rather than silently
  no-op; the byte-compatible writer is a later milestone.
- New `tests/file_backed.rs` (rusqlite dev-dep oracle): builds a genuine `.sqlite`
  file, opens it via `connect(path)`, and asserts `SELECT` (projection, `WHERE`,
  `ORDER BY`, aggregates, and the `INTEGER PRIMARY KEY` rowid alias) returns what
  the real library does — proving the whole engine runs over a real file.

This graduates the Rust port past the old Level-0 rule ("file-backed connections
raise `NotSupportedError`", conformance fixture 12); the fixture's
`connect_expect_error` still holds because a *missing* file still errors.

## 0.3.0 — Differential conformance oracle (baseline)

First step of the roadmap to make mini-sqlite a drop-in SQLite replacement
(`code/specs/mini-sqlite-full-conformance.md`, Stream A / L2).

- **`tests/differential_oracle.rs`** — a differential-conformance harness that
  runs the same SQL through mini-sqlite *and* real bundled SQLite (`rusqlite`, a
  **dev-dependency only** — never linked by the shipped crate) and asserts they
  agree: matching columns (case-insensitive), matching rows (order-sensitive only
  under `ORDER BY`), and error-vs-success agreement. This is the measuring
  instrument the whole conformance roadmap is gated on, mirroring the
  `sqlite-file` crate's cross-check against the real on-disk format.
- On introduction it measured **12 of 22 seed cases already matching SQLite** and
  reproduced **10 genuine gaps**, recorded in an explicit known-divergence
  `LEDGER` (shrinking the ledger is the conformance metric): qualified column
  refs across a join resolve to NULL (breaks even `INNER JOIN`); `LEFT`/`RIGHT`/
  `FULL JOIN` drop their `ON` clause; aggregate columns are misnamed (`agg_N` vs
  `SUM(n)`); and `UPPER()` returns the wrong value. Each is a tracked follow-up
  increment. No shipped code changed in this release — only the test harness.

## 0.2.1 — Security hardening (post-review fixes)

- `quote_sql_string`: strip NUL bytes from `Text` parameters before escaping.
  Some lexers treat an embedded `\0` as a string terminator, which could allow
  a malicious value to inject raw SQL after the NUL.
- `read_quoted`: fix `''` doubled-single-quote handling in the parameter
  scanner.  Previously the scanner exited on the first `'` of a `''` escape
  sequence, causing downstream `?` placeholders to be mis-positioned.
- `sql-vm — SaveGroupKey`: cap GROUP BY distinct keys at 1 000 000.  Queries
  over high-cardinality GROUP BY columns could exhaust memory; now returns
  `VmError::ResourceLimit` instead.
- `sql-vm — CountDistinct`: cap distinct-value set at 1 000 000 entries.
  `COUNT(DISTINCT large_blob_col)` over millions of rows could accumulate
  gigabytes of hex strings; now returns `VmError::ResourceLimit` instead.
- `sql-vm — VmError`: add `ResourceLimit(String)` variant used by the two new
  caps above.

## 0.2.0 — Level 1 graduation (full pipeline)

Route ALL SQL — DDL, DML, and SELECT — through the complete Mini-SQLite
pipeline instead of the hand-rolled Level 0 executor:

```
sql-lexer → sql-parser → sql-planner → sql-optimizer → sql-codegen → sql-vm
```

All 45 conformance + unit tests pass. Key changes in the pipeline:

### mini-sqlite facade
- Replace `InMemoryDatabase` (Level 0 hand-rolled store) with
  `coding-adventures-sql-backend::InMemoryBackend` as the storage layer.
- Remove `coding-adventures-sql-execution-engine` dependency; SELECT now
  goes through the same pipeline as INSERT/UPDATE/DELETE/DDL.
- Add `SchemaProvider` adapter (`backend_as_schema_provider`) so the planner
  can resolve table schemas against the live backend.
- Transaction management: `begin_transaction` is called automatically on the
  first DML/DDL within a connection; `commit()` and `rollback()` delegate to
  the backend's transaction handles.
- Add `serde_json` dev-dependency and a JSON-driven conformance test runner
  that loads all 24 fixtures from `code/specs/mini-sqlite-conformance/fixtures/`.
- `sql_literal()` ensures float parameters include a decimal point.
- Parameter binding (`bind_parameters`): qmark-style `?` substitution.
- `MiniSqliteError` variants unchanged; `API_LEVEL`, `THREAD_SAFETY`,
  `PARAM_STYLE` constants unchanged.

### sql-lexer
- Add `CONCAT_OP` token (`||`) declared before any single-pipe token so the
  lexer always prefers the longer two-character match.

### sql-parser
- Add `||` to the `additive` rule so `||` is parsed as a binary operator.
- Make `FROM table_ref { join_clause }` optional in `select_stmt` so
  `SELECT expr` without a FROM clause is accepted.
- Add optional `-` before `NUMBER` in `limit_clause` for `LIMIT -1`.
- Allow optional `DISTINCT` before args in `function_call` for `COUNT(DISTINCT col)`.

### sql-planner
- Support `SELECT expr` without FROM via a `__dual__` virtual table (`SELECT 1 + 1`).
- `plan_limit()` handles `LIMIT -1` (returns `count = Some(-1)` = all rows).
- HAVING aggregate deduplication: `COUNT(*)` in HAVING reuses the SELECT slot.

### sql-optimizer
- `Project(EmptyResult)` no longer collapses to `EmptyResult` — the Project's
  column list encodes the output schema needed for `DefineColumns`.

### sql-codegen
- Add `AggFn::CountDistinct` variant for `COUNT(DISTINCT col)`.
- Add `Instruction::DefineColumns(Vec<String>)` — sets output column names
  without emitting rows (for `LIMIT 0` queries).
- Add `Instruction::CallBuiltin(String, usize)` — dispatches named scalar
  SQL built-in function calls (LENGTH, UPPER, LOWER, TRIM, SUBSTR, REPLACE,
  ABS, COALESCE, ROUND).
- Compile `Project(EmptyResult)` → `DefineColumns(col_names)` instead of nothing.
- Compile `FunctionCall` → `CallBuiltin` instead of `LoadConst(Null)`.
- Add `agg_slots` field to `Compiler` so `compile_expr` emits `FinalizeAgg`
  with the correct slot index in multi-aggregate HAVING predicates.

### sql-vm
- Add `__dual__` virtual table support in `OpenScan`: yields one empty row
  for `SELECT expr` without FROM.
- Fix `BinaryOp::Concat` to propagate NULL (was converting NULL to empty string).
- Add `DefineColumns` instruction handler: sets output column names without
  emitting rows.
- Add `CallBuiltin` instruction handler with full `call_builtin()` dispatcher.
- Implement GROUP BY multi-group aggregation: `SaveGroupKey` now tracks
  per-group accumulators; `CloseScan` triggers group iteration mode; the
  finalize/predicate/emit block is re-executed once per group.
- Implement `AggFn::CountDistinct` accumulator with lazy `HashSet<String>`.
- Fix `apply_limit` for `LIMIT -1`: negative count = all rows (SQLite semantics).
- Fix `UpdateRows` to carry column names for multi-column SET assignments.
- HAVING + GROUP BY: `JumpIfFalse` advances group iterator on predicate-false
  instead of jumping to the skip label prematurely.

## 0.1.0

- Add a Level 0 Rust mini-sqlite facade backed by in-memory tables.
- Support DB-API-inspired connection and cursor methods, qmark binding,
  snapshot commit/rollback, and SELECT delegation through the Rust SQL
  execution engine.
