# Changelog

## 0.5.63 — `SELECT *` (and `SELECT DISTINCT *`) expand to real columns

`SELECT * FROM t` now returns the table's actual columns instead of a single
NULL column named `*`. Previously `*` survived planning as a placeholder that no
downstream stage could resolve (`LoadColumn("*")` → NULL). The planner now
expands `*` into the base table's columns in declaration order — matching SQLite
— before DISTINCT-collation and ORDER-BY-ordinal resolution, so:

- `SELECT DISTINCT *` folds each column under its own declared collation (a
  `COLLATE NOCASE` column dedupes case-insensitively) — retires the
  `distinct_star_collate` ledger entry.
- `SELECT * FROM t ORDER BY 1` binds the ordinal to the first expanded column,
  as SQLite does (new `select_star_order_by_ordinal` oracle case).

Scoped to a single base table with no JOIN; joined `*` keeps the placeholder
(separate gap). `*` mixed with other items (`SELECT a, *`) is newly ledgered as
`select_star_in_place` — it does not yet PARSE (the grammar's select_list has no
bare-`*` alternative in a comma list); the planner already handles that shape
once the parser produces it. Verified against bundled real SQLite. Spans
sql-planner 0.2.30.

## 0.5.62 — GROUP BY aggregate columns follow the SELECT list

`SELECT max(x) AS mx, c FROM t GROUP BY c` now returns columns `[mx, c]` — the
SELECT-list order — where before an aggregate column was always emitted after
the group keys in a fixed layout (`[c, max(x)]`). This completes the GROUP BY
projection work: non-aggregate columns already followed the SELECT list, and now
aggregate columns do too, including `group_concat`.

The enabler is reconciling `group_concat`'s representation: the planner now
lowers it to `SqlExpr::Aggregate` like COUNT/SUM (it was previously a
`FunctionCall` in the SELECT list), so codegen can re-compile any aggregate
column in place. Retires the `group_by_reordered_with_aggregate` ledger entry.
Verified against bundled real SQLite, including group_concat with a separator,
count, and key all reordered. Spans sql-planner 0.2.29 and sql-codegen 0.6.11.

## 0.5.61 — GROUP BY bare column takes the group's first-row value

`SELECT c FROM t GROUP BY x`, where `c` is neither a GROUP BY key nor inside an
aggregate, now returns a value instead of NULL — SQLite reports such a bare
column from the group's first row, and the VM now retains a representative row
per group to do the same. Retires the `distinct_over_group_by_single_col` ledger
entry.

Verified against bundled real SQLite, including a multi-row group where the bare
columns come from the first row (`v=10, tag='a'`) and a functionally-dependent
bare column (deterministic regardless of which row is picked). The min/max-
follows refinement (bare columns tracking a `min()`/`max()` row) stays out of
scope — it needs an aggregate combined with bare columns, still ledgered.

## 0.5.60 — GROUP BY output follows the SELECT list

`SELECT x, c FROM t GROUP BY c, x` now returns columns in SELECT-list order
`[x, c]`, and `SELECT c FROM t GROUP BY x` returns a column named `c` — where
before, GROUP BY output always came back as the group-key columns in GROUP BY
order followed by the aggregates, so reordering or renaming the SELECT list had
no effect (and column identity could be flat wrong). This was the root cause of
the data-loss near-miss the DISTINCT-collation security review caught.

Retires the `distinct_over_group_by_no_misfold` ledger entry. Two narrower gaps
remain, now ledgered precisely: a bare non-key column (`SELECT c ... GROUP BY x`)
projects under the right name but as NULL, since the group keeps no
representative source row (SQLite's bare-column extension); and an aggregate
column reordered before a group key still uses the fixed layout, pending
reconciliation of how `group_concat` is represented in the SELECT list. Verified
against bundled real SQLite, including two new positive cases (a plain reorder
and an expression over a group key).

## 0.5.59 — ORDER BY alias no longer borrows a column's COLLATE

`SELECT x AS c FROM t ORDER BY c` sorted case-insensitively when the table
happened to contain an unrelated `c TEXT COLLATE NOCASE` column — the ORDER BY
key resolved the bare name against the base table instead of the output list, so
the alias inherited a collating sequence from a column the query never used.
Now the collation follows what the alias stands for: `x AS c ... ORDER BY c` is
byte order, `c AS y ... ORDER BY y` still folds NOCASE. Verified against
bundled real SQLite with all three shapes (shadowing alias, alias of a collated
column, and a genuine collated-column reference as a control).

## 0.5.58 — DISTINCT honours a column's declared COLLATE

`SELECT DISTINCT c` on a column declared `COLLATE NOCASE` now dedupes
case-insensitively, matching SQLite, and the surviving row keeps its ORIGINAL
text ('A' when that row came first). Retires the `distinct_column_collate_nocase`
ledger entry.

The fold applies only where SQLite applies it — to a bare column reference.
Verified against real SQLite and locked in by oracle cases: `c` folds, `c AS y`
folds (an alias is just a label), `x AS c` does NOT fold (the name is irrelevant;
`x` is BINARY), and `c||''` does NOT fold (expressions drop the collation).

Newly recorded in the ledger: `SELECT DISTINCT *` does not expand the star into
the table's columns — a projection gap unrelated to collation, and the first
case to exercise `DISTINCT *` at all.

## 0.5.57 — GROUP BY honours a column's declared COLLATE

`GROUP BY c` on a column declared `COLLATE NOCASE` now groups case-insensitively,
matching SQLite: `'A'` and `'a'` land in one group. Crucially the collation folds
only the grouping KEY — each group still reports its ORIGINAL text, so a group of
`{'A','a'}` shows `'A'` when that row came first (not the case-folded `'a'`), under
its real column name. Restricted to a single base table (no JOINs), matching the
existing ORDER BY / WHERE collation passes; an explicit `COLLATE` on the key still
outranks the declared one.

`DISTINCT` does not yet fold on a declared collation, and an explicit `COLLATE`
suffix in a DISTINCT select-item or GROUP BY term does not parse yet — all three
are recorded as known divergences in the oracle ledger and are the next
increments in this lane.

## 0.5.56 — arithmetic operands keep real-syntax text REAL (`'9.0'/2` = 4.5)

Text operands in arithmetic were coerced with SQLite's `CAST(… AS NUMERIC)` rule,
which collapses an integral real to an integer — so `'9.0' / 2` did integer
division and returned `4` where SQLite returns `4.5`. Arithmetic operands use a
*different* rule (`applyNumericAffinity`): the type follows the text's **syntax**,
not its value. `sql-vm` now applies that rule (new `text_to_numeric_operand`) in
binary `+ - * / %` and unary minus, leaving `CAST(… AS NUMERIC)` untouched.

Verified against the real `sqlite3` binary — `'9.0'/2`→`4.5` real, `'9'/2`→`4`
integer, `'1e2'+0`→`100.0` real, `-'3e2'`→`-300.0` real, while `'3e'`/`'3e+'`
(incomplete exponent) stay integer `3` and `'abc'`/`'.'` stay integer `0`. Three
new differential-oracle cases cover the fix, the prefix-syntax boundaries, and the
deliberate CAST-vs-operand contrast; the oracle was confirmed to *fail* without the
fix. Retires the last documented item of the type-affinity arc.


## 0.5.55 — `i64::MIN` div/mod overflow + integer `%`

Two `%` / `/` edge cases now match SQLite. **`i64::MIN` overflow:**
`0x8000000000000000 / -1` promotes to REAL (`9.2233720369e18`) instead of
erroring, mirroring the `+`/`-`/`*` overflow promotion, and
`0x8000000000000000 % -1` returns INTEGER `0` (its true remainder). **Integer
`%`:** SQLite's modulo is an integer operation — both operands are truncated
toward zero to 64-bit integers before the remainder is taken, and the result is
REAL only if an operand was REAL. So `7.5 % 2` is now `1.0` (7 % 2), not `1.5`
(fmod); `10.9 % 3.9` is `1.0` (10 % 3); a real divisor that truncates to zero
(`5 % 0.9`) is NULL. Division (`/`) is unchanged — it stays true real division
(`7.5 / 2` = 3.75). Implemented in `sql-vm`'s `Div`/`Mod` arms; verified against
bundled real SQLite in the differential oracle.

## 0.5.54 — Hexadecimal integer literals

`SELECT 0x1F` now works (was a parse error). SQLite hex integer literals like
`0x1F` (= 31), `0X10` (= 16), and `0xff` (= 255) are recognised as INTEGERs
(`typeof(0x1F)` = `'integer'`). They decode as a 64-bit value that wraps, so
`0x7FFFFFFFFFFFFFFF` is `i64::MAX` and `0xFFFFFFFFFFFFFFFF` is -1, matching
SQLite. Hex literals compose in arithmetic and predicates like any integer
(`v + 0x10`, `WHERE v = 0x1F`). More than 16 hex digits overflows 64 bits and is
rejected, as SQLite does ("hex literal too big"). Implemented by a new `HEX_INT`
token in `sql-lexer` (matched before `NUMBER` so the whole `0x…` is one token,
aliased to `NUMBER` so no parser rule changed) plus `0x`-prefix decoding in the
`sql-planner` number-literal decoder. Verified against the bundled real SQLite in
the differential oracle.

## 0.5.53 — GROUP_CONCAT aggregate

`SELECT group_concat(x) FROM t` now works (was "unknown built-in function"). It
concatenates the non-NULL values of `x` in row order, joined by a separator that
defaults to `,` and can be overridden by a literal 2nd argument (e.g.
`group_concat(x, '|')`, or `''` for no separator). NULLs are skipped, an empty or
all-NULL group is NULL (not an empty string), and `group_concat(DISTINCT x)`
deduplicates values before joining — all matching SQLite. Spans sql-planner
0.2.24 (aggregate recognition + separator capture), sql-codegen 0.6.7
(`AggFn::GroupConcat`), and sql-vm 0.4.30 (accumulation + dedup + DoS caps).
Three new differential-oracle cases. Known follow-ups (documented, not yet
matched): `DISTINCT` compares by storage class so `1` and `1.0` dedup separately
where SQLite collapses them numerically (shared with `COUNT(DISTINCT)`); a
non-string-literal separator falls back to `,`; and `group_concat(DISTINCT x,
sep)` is accepted where SQLite rejects it.

## 0.5.52 — integer arithmetic overflow promotes to REAL

`SELECT 9223372036854775807 + 1` now returns `9.2233720369e18` (real) instead of
erroring. When the exact `i64` result of `+`/`-`/`*` or unary `-` overflows,
SQLite redoes the operation in floating point and yields a REAL — never an error
or a wrap. `min_i64 - 1`, `max_i64 * 2`, and `-(min_i64)` all promote likewise;
`typeof` of an overflowed result is `'real'`. Non-overflowing arithmetic still
returns INTEGER. Fixed in sql-vm 0.4.29 (`checked_int_binop` + unary `Neg`). One
new differential-oracle case. (The `i64::MIN / -1` division/modulo edge, which is
entangled with SQLite's integer-coercing `%` operator, is a separate follow-up.)

## 0.5.51 — scientific-notation numeric literals

`SELECT 1e3` now parses (→ `1000.0`), along with `2.5e2`, `1.5e-3`, `10e+2`, and
`1E2*2`. A numeric literal carrying an exponent is REAL, so `typeof(1e3)` is
`'real'` even though the value is integral — matching SQLite. Previously the
lexer's `NUMBER` token had no exponent, so `1e3` mis-lexed as `1` + a `NAME`
`e3` and failed to parse. The fix is a one-line lexer regex extension (sql-lexer
0.1.4); the planner already decoded exponent forms to REAL. Ordinary subtraction
(`5-3` = 2) is unaffected. Two new differential-oracle cases.

## 0.5.50 — explicit COLLATE before IN / BETWEEN / LIKE

`x COLLATE NOCASE IN (…)` now parses and applies the collation to the membership
test, matching SQLite: an explicit collation on the left operand drives every
equality the `IN` performs, lifting a plain column to case-insensitive membership
or overriding a column's declared sequence (`COLLATE BINARY` forces byte order).
It composes with IN's three-valued NULL logic. Previously the grammar accepted
`COLLATE` only before a comparison operator (`=`, `<`, …), so `COLLATE` before
`IN` was a parse error. Grammar prefix added in sql-parser 0.1.21; planner lowers
it to `__collate` wraps in sql-planner 0.2.23. New differential-oracle cases.

Also in this version: **`x COLLATE NOCASE BETWEEN a AND c`** parses and applies
the collation to the inclusive range test (both `>= a` and `<= c`); and
**`COLLATE` before `LIKE`/`GLOB`** now parses but is *ignored* (validated then
discarded), matching SQLite — LIKE/GLOB carry their own case-folding, so even
`COLLATE BINARY` does not make LIKE case-sensitive. This completes the
explicit-`COLLATE`-before-`IN`/`BETWEEN`/`LIKE`/`GLOB` surface.

**Bug fix (sql-vm 0.4.28): `NOT BETWEEN` now returns the logical negation** of
the inclusive range rather than a strict/exclusive-bounds test. `5 NOT BETWEEN 1
AND 10` was wrongly `1` (5 is in `[1,10]`, so it is `0`); `15 NOT BETWEEN 1 AND
10` was wrongly `0`. This latent bug had never been exercised by the oracle —
two cases now guard it — and it was surfaced by the collated `NOT BETWEEN` work.

## 0.5.49 — text truthiness takes numeric affinity

`NOT 'abc'` now returns 1 (not 0) and `WHERE <text>` keeps only rows whose text
is numerically non-zero, matching SQLite. A text/blob in a boolean context takes
numeric affinity: `'abc'`/`'0'`/`''` are false, `'5'`/`'5.5'` are true. This
completes the type-affinity work — the boolean-context analog of the arithmetic
(0.5.48) and unary-minus (0.5.43) coercion — and applies uniformly to WHERE,
HAVING, AND/OR, NOT, IIF, and CASE WHEN. Two new differential-oracle cases;
sql-vm 0.4.27.

## 0.5.48 — arithmetic coerces text/blob operands

`'5' + 0` now returns 5 instead of erroring. Binary arithmetic (`+ - * / %`)
applies SQLite numeric affinity to text/blob operands: `'abc' + 1` = 1, `'10' -
'3'` = 7, `'5' * 2` = 10, `5 / '2'` = 2, `5 / '0'` = NULL, `'7' % 3` = 1. This is
the binary analog of the unary-minus coercion (0.5.43). Comparison and bitwise
operators are unaffected. Two new differential-oracle cases; sql-vm 0.4.26.
Known edge (shared with unary minus): an integral real-syntax string `'9.0'`
collapses to an integer, a float-affinity follow-up.

## 0.5.47 — IN uses numeric equality + three-valued NULL

`1 IN (1.0)` now returns 1 (true) instead of 0: `IN` membership uses the same
equality as `=`, so INTEGER and REAL compare numerically (`'1' IN (1)` stays
false — text vs integer). `IN` is also now correctly three-valued for NULL: a
NULL list element makes an otherwise-non-matching test NULL (`1 IN (NULL,2)` →
NULL), while a real match still wins (`1 IN (NULL,1)` → 1); `NOT IN` inverts.
Two new differential-oracle cases; sql-vm 0.4.25. The IN-collation folding
(0.5.44) is unaffected.

## 0.5.46 — `||` concatenates blobs as raw bytes

`X'41' || 'B'` now returns `'AB'` (0x41 = 'A'), not `"x'41'B"`. The `||` operator
was rendering a blob operand in its `x'…'` hex *display* form; it now uses the
blob's raw bytes as text, matching SQLite (the result is TEXT). blob||text,
text||blob, and blob||blob all fold to the byte string. One new
differential-oracle case; sql-vm 0.4.24. (The `1 IN (1.0)` numeric-equality gap
noted alongside this remains a separate follow-up.)

## 0.5.45 — LENGTH() over blobs and numbers

`LENGTH(x'0102ff')` now returns 3 (the byte count) instead of erroring, and
`LENGTH(12345)` returns 5 (decimal-text length). LENGTH previously accepted only
text/NULL; now a blob measures its raw bytes (distinct from text's *character*
count) and a number its text-form length, matching SQLite. Blob literals (0.5.42)
made the blob case reachable from SQL. Two new differential-oracle cases; sql-vm
0.4.23. Floats remain declined (subtle text form).

## 0.5.44 — column COLLATE honored in the IN operator

A column declared `COLLATE NOCASE` / `RTRIM` now folds case (etc.) in `IN`
membership, not just `=`/`<`/… comparisons: `SELECT id FROM t WHERE name IN
('APPLE')` on a NOCASE `name` matches both `'Apple'` and `'apple'`. `NOT IN`
inverts it, multi-element lists fold every element, and an explicit `COLLATE
BINARY` on the value overrides the column's NOCASE. NULL semantics are
unchanged. Four new differential-oracle cases; sql-planner 0.2.22 (single base
table, matching the existing WHERE-comparison restriction). `DISTINCT` and
`GROUP BY` collation remain follow-ups.

## 0.5.43 — unary minus coerces text/blob operands

`-'5'` now returns `-5`, not the string `'5'`. SQLite applies numeric affinity to
the operand of unary minus before negating; the engine previously left text
unchanged. Now `-'12abc'` = -12 (leading numeric prefix), `-'abc'` = 0, `-'3.5'`
= -3.5, and leading whitespace is tolerated (`-'  7'` = -7). One new
differential-oracle case + a sql-vm unit test; sql-vm 0.4.22. An exponent-form
string (`-'3e2'`) is a documented edge left for a float-affinity follow-up.

## 0.5.42 — blob literals `x'…'` parse

SQL blob literals `x'48656C6C6F'` / `X'FF00'` now parse end to end. Previously
the lexer split `x'414243'` into a `NAME` and a `STRING` and the query failed;
now the whole literal is one token that the planner decodes into raw bytes.
`HEX(x'414243')` is `'414243'`, `TYPEOF(x'00')` is `'blob'`, `QUOTE(X'DEADBEEF')`
is `X'DEADBEEF'`, and `WHERE b = x'0102'` filters by byte-exact blob equality.
The empty literal `x''` is the zero-byte blob; an odd number of hex digits is a
parse error, as in SQLite. The VM already supported `Blob` values, so this is a
pure front-end (lexer→parser→planner) fix. Touches sql-lexer 0.1.3, sql-parser
0.1.20, sql-planner 0.2.21. `LENGTH()` over a blob is a documented follow-up.

## 0.5.41 — column-defined COLLATE in WHERE comparisons

A column declared `COLLATE NOCASE` / `RTRIM` now folds in WHERE comparisons, not
just ORDER BY: `SELECT id FROM t WHERE name = 'apple'` on a NOCASE `name` matches
both `'Apple'` and `'apple'`, and RTRIM ignores trailing spaces (`'hi   ' =
'hi'`). Comparison operators `=`, `<>`, `<`, `<=`, `>`, `>=` all honour the
column's sequence, and it flows through `AND`/`OR`/`NOT`. An explicit `COLLATE
BINARY` on the comparison overrides the column's NOCASE (byte-exact match), and a
column without a declared collation stays BINARY. Seven new differential-oracle
cases pass against real SQLite. `DISTINCT`, `GROUP BY`, and `IN` collation remain
follow-ups. Touches sql-planner 0.2.20 (single base table only, matching the
ORDER BY restriction).

## 0.5.40 — ORDER BY positional (ordinal) column references

`ORDER BY <n>` now treats a bare integer as a 1-based reference to the n-th
SELECT output column, matching SQLite: `SELECT a, b FROM u ORDER BY 2` sorts by
`b`. Direction, collation, and multi-key tie-breaks carry through
(`ORDER BY 2 DESC, 1`). The rule is narrow — only a lone integer literal is
positional, so `ORDER BY 1+0` still sorts by the constant `1` (no reordering),
and an out-of-range ordinal errors like SQLite's "ORDER BY term out of range".
Four new differential-oracle cases pass; a fifth — positional over an *aggregate*
output column — is a documented ledger entry (the sort can't re-evaluate an
aggregate per row yet). `SELECT *` positional keys are left unchanged for now.
Touches sql-planner 0.2.19.

## 0.5.39 — upper()/lower() are ASCII-only

`upper()` and `lower()` now match SQLite: they case-fold only ASCII letters and
leave accented / non-Latin characters untouched — `upper('naïve')` is `'NAïVE'`
and `upper('straße')` is `'STRAßE'`. Two differential-oracle cases; sql-vm 0.4.21.

## 0.5.38 — LIKE … ESCAPE, and NOT LIKE inversion fix

`X LIKE Y ESCAPE Z` now parses and evaluates: the escape character makes a
following `%`, `_`, or itself a literal in the pattern (e.g. `'100%' LIKE '100#%'
ESCAPE '#'`). While wiring the negation through, this also fixes a pre-existing
bug where **`NOT LIKE` behaved exactly like `LIKE`** (the `NOT` was dropped);
`NOT LIKE` now correctly inverts the match, with `NULL` operands staying `NULL`.
Three differential-oracle cases; touches sql-parser 0.1.19, sql-planner 0.2.18,
sql-codegen 0.6.6, sql-optimizer 0.1.5, sql-vm 0.4.20. (Note: `ESCAPE '\'` with
a *string-literal* pattern is still affected by a separate lexer bug that strips
backslashes in string literals — tracked separately; column-valued patterns and
non-backslash escape characters work.)

## 0.5.37 — substr() start-zero and negative-length edge cases

`substr()` now matches SQLite on its two fiddly edges: `substr('hello',0)` is
`'hello'` (Y=0 is a virtual slot before the first character) and
`substr('hello',2,-1)` is `'h'` (a negative length returns the characters
*preceding* the start, reading leftward) — both previously returned `''`. Still
character-based for multibyte UTF-8. Four differential-oracle cases; sql-vm
0.4.19.

## 0.5.36 — CAST … AS NUMERIC

`CAST(x AS NUMERIC)` (and NUMERIC-affinity type names like `DECIMAL`, `BOOLEAN`)
now work instead of erroring. NUMERIC prefers INTEGER when the value is integral
and fits i64 — `CAST('3.0' AS NUMERIC)` → `3`, `CAST('1e3' AS NUMERIC)` → `1000`
— and REAL otherwise (`CAST('3.5' AS NUMERIC)` → `3.5`; an i64-overflowing
integer → real). A number is left unchanged: `CAST(3.0 AS NUMERIC)` stays `3.0`.
Six differential-oracle cases; sql-planner 0.2.17, sql-vm 0.4.18. (`CAST … AS
BLOB` is still rejected — a spun-off follow-up.)

## 0.5.35 — Division / modulo by zero returns NULL

`SELECT 5/0`, `5.0/0`, `0/0`, `5%0`, `5.5%0`, and `5%0.0` now return **NULL**
like real SQLite, instead of failing the query with a "division by zero" error.
This makes per-row expressions with an occasional zero divisor
(`SELECT 100/n FROM t`) behave as SQLite does — the zero-divisor row is NULL and
the rest compute normally. Four differential-oracle cases added; sql-vm 0.4.17.
(Follow-ups: `%` integer-remainder semantics and unary `-` numeric coercion on
text are tracked separately.)

## 0.5.34 — Column-defined COLLATE honoured in ORDER BY

A column declared with `COLLATE NOCASE` / `RTRIM` now drives ordering when a
query sorts by that column without an explicit COLLATE:
`CREATE TABLE t(id INTEGER, name TEXT COLLATE NOCASE); SELECT name FROM t ORDER
BY name` folds case exactly like real SQLite (previously the column collation
was parsed and discarded, so the sort fell back to BINARY). An explicit COLLATE
on the ORDER BY key still wins, and an unknown collation on the column is
rejected at CREATE time. Adds seven differential-oracle cases (sql-parser 0.1.18,
sql-planner 0.2.16, sql-backend 0.1.1). Comparison / GROUP BY / DISTINCT
collation and multi-table resolution remain follow-ups.

## 0.5.33 — Scalar subquery `( SELECT … )` parses (evaluation is a follow-up)

`SELECT (SELECT count(*) FROM t2)` and `WHERE x > (SELECT max(y) FROM t2)` now
PARSE — the grammar's `primary` accepts a parenthesised `SELECT` (sql-parser
0.1.17), tried before the plain `( expr )` form. Evaluation is not yet wired: the
planner rejects a scalar subquery with a clear
`scalar subqueries are not yet supported` error (sql-planner 0.2.15) rather than
panicking or mis-planning — an integration test confirms the parse→plan→clean-error
path end to end. This is the first, parse-only slice of scalar-subquery support;
a follow-up adds `SqlExpr::ScalarSubquery` + the VM sub-plan evaluation (run the
inner SELECT once, take row 0 / column 0, NULL if empty). The plain
`( 1 + 2 )` parenthesised-expression form is unaffected (regression-tested).

## 0.5.32 — `COLLATE` on the left comparison operand

`x COLLATE NOCASE = y` now folds the comparison just like `x = y COLLATE C`
(and `WHERE name COLLATE NOCASE = 'foo'` matches per row). Grammar-only
(sql-parser 0.1.16): the left-side COLLATE binds inside the `cmp_op` alternative,
so it never steals an `ORDER BY … COLLATE …` clause's collation (that regression
was caught by the differential oracle and fixed via ordered-choice backtracking).
The planner reuses the merged `collate_name_after`/`wrap_collate` unchanged — the
first COLLATE token wins, so a left collation takes precedence over a right one,
matching SQLite. Two differential-oracle cases (`=` and a WHERE predicate) plus a
parser regression test for ORDER BY COLLATE.

## 0.5.31 — `IS [NOT] DISTINCT FROM` (standard-SQL null-safe compare)

`SELECT a IS DISTINCT FROM b`, `a IS NOT DISTINCT FROM b`, and the WHERE-predicate
form now parse and run: `IS NOT DISTINCT FROM` is the null-safe equality (both-NULL
is "not distinct" = equal, one-NULL distinct, values compared otherwise) and `IS
DISTINCT FROM` is its negation — the SQL-standard spelling of the `IS`/`IS NOT`
operator merged earlier. It is lowered entirely in the planner (sql-parser 0.1.15
grammar; sql-planner 0.2.14 inverts the sense on DISTINCT) onto the existing
`plan_is_distinct` null-safe-CASE machinery — no new codegen or VM. Two
differential-oracle cases (the four-combination table + a WHERE predicate) diff
against real bundled SQLite.

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
