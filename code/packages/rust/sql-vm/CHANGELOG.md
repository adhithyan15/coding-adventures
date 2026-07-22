# Changelog — coding-adventures-sql-vm

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.4.35] - Unreleased

### Added

- **A GROUP BY over a bare non-key column reports the group's first-row value,
  not NULL.** `SELECT c FROM t GROUP BY x` (where `c` is neither a GROUP BY key
  nor inside an aggregate) returned NULL; SQLite reports such a bare column from
  the group's FIRST row. The GROUP BY state now keeps a representative row per
  group — the first source row, snapshotted when the group is created — and the
  Phase-2 fake row is built from it (via `build_group_fake_row`) with the
  canonical key-column values overlaid on top, so a bare column resolves to a
  value while a collated key still reports its original text. Both engines scan
  an in-memory table in rowid order, so "first row of the group" is
  deterministic. (The min/max-follows refinement — bare columns tracking the row
  that holds a `min()`/`max()` — needs an aggregate present, which the projection
  path does not yet combine with bare columns; a separate ledgered gap.)

## [0.4.34] - Unreleased

### Added

- **`apply_distinct` honours per-output-column collations.** A DISTINCT column
  declared `COLLATE NOCASE` now folds 'A' and 'a' to one dedupe key. Only the KEY
  is folded — `retain` keeps the FIRST matching row, so the surviving row still
  carries its ORIGINAL text, matching SQLite. Collation applies to TEXT only. A
  short (or empty) collation slice leaves the remaining columns at BINARY, the
  stricter fail-safe direction.

## [0.4.33] - Unreleased

### Fixed

- **Group-key separator injection could merge two distinct GROUP BY groups.**
  The multi-column group key joins per-column `"t:<value>"` segments with `\x1F`;
  because TEXT can hold arbitrary bytes, a value containing that separator
  followed by a type tag forged a segment boundary, so two DIFFERENT key tuples
  serialised identically — they collapsed into one group and the first tuple's
  values were reported for both (the other row's data went missing and was
  misattributed). TEXT segments are now LENGTH-PREFIXED (`t:<byte-len>:<text>`),
  so a separator inside the counted region is unambiguously data. Found by the
  security review of the GROUP BY collation work; the value is attacker-
  controlled, so this was a real data-integrity bug rather than a theoretical
  one. The other segment kinds are self-delimiting (fixed alphabets) and are
  unchanged.

### Added

- **`SaveGroupKey` honours per-key collations.** The group key string is now
  built from collation-folded TEXT (via `collate_text`) when a key column
  declares a collating sequence, so `GROUP BY c` on a `COLLATE NOCASE` column
  puts `'A'` and `'a'` in one group. Only the key string is folded — the
  original values stay in `key_vals` and are what the group reports, matching
  SQLite (a group of `{'A','a'}` reports `'A'` when that row came first).
  Collation applies to TEXT only; numbers, blobs and NULL have no collating
  sequence in SQLite.

## [0.4.32] - Unreleased

### Fixed

- **Arithmetic operands keep the type their syntax implies — `'9.0' / 2` is now
  `4.5`, not `4`.** Arithmetic was applying SQLite's `CAST(… AS NUMERIC)` rule
  (`text_to_numeric`) to text/blob operands, which deliberately *collapses* an
  integral real to an integer (`CAST('3.0' AS NUMERIC)` really is the integer `3`).
  But an *arithmetic operand* uses a different SQLite rule (`applyNumericAffinity`):
  the result type follows how the text is **written**, never whether the value
  happens to be integral. So `'3.0' + 0` is the real `3.0`, and real division
  applies.
  - New `text_to_numeric_operand` implements the operand rule and is now used by
    `coerce_arith` (binary `+ - * / %`) and unary minus. `text_to_numeric` is
    unchanged and still backs `CAST(… AS NUMERIC)` — the two rules are deliberately
    different and a test now pins that difference.
  - The prefix boundaries were verified against the real `sqlite3` binary: a `.`
    anywhere (`'3.0'`, `'3.'`, `'.5'`) or a **complete** exponent (`'1e3'`,
    `'3e2x'`) makes it REAL; an **incomplete** exponent is not consumed, so `'3e'`
    and `'3e+'` stay the integer `3`; digitless text (`'abc'`, `'.'`, `'-'`, `''`)
    is integer `0`; integer syntax overflowing `i64` promotes to REAL.
  - Fixes the "float-affinity edge" previously documented in-code as a known
    divergence for both binary arithmetic and unary minus (`-'3e2'` is now
    `-300.0`). 3 new unit tests; 3 new differential-oracle cases.


## [0.4.31] - Unreleased

### Changed

- **`i64::MIN` division/modulo overflow now matches SQLite instead of erroring.**
  `i64::MIN / -1` has no `i64` representation, so it now PROMOTES to REAL
  (`9223372036854775808.0`), mirroring the existing `+`/`-`/`*` overflow
  promotion. `i64::MIN % -1` (the only overflow `%` can hit) returns INTEGER `0`,
  its true remainder. Both previously surfaced a `VmError` ("integer overflow in
  division/modulo").
- **`%` is now an INTEGER operation, matching SQLite.** Both operands are
  converted to 64-bit integers (numeric affinity, then truncation toward zero,
  with out-of-range reals clamped to the i64 bounds via Rust's saturating
  `as i64` float cast) before the remainder is taken; the result is REAL only if
  an operand carried REAL affinity. So `7.5 % 2` is now `1.0` (7 % 2), not `1.5`
  (fmod), and `10.9 % 3.9` is `1.0` (10 % 3). A real divisor that truncates to
  zero (`5 % 0.9`) is NULL, as before. Division (`/`) is unchanged — it stays
  true real division (`7.5 / 2` = 3.75).

## [0.4.30] - Unreleased

### Added

- **`GROUP_CONCAT` aggregate execution.** `update_accumulator`/
  `finalize_accumulator` handle `AggFn::GroupConcat { sep, distinct }`: non-NULL
  values are rendered with `sql_to_str` and appended, joined by `sep`; an empty or
  all-NULL group finalises to NULL. `DISTINCT` deduplicates using the same
  type-tagged key as `COUNT(DISTINCT)`, now factored into a shared `distinct_key`
  helper. Values are appended IN PLACE (`push_str`, amortised O(total length))
  rather than rebuilt with `format!` each row (which would be O(n²) and double
  peak memory). Two DoS guards: the result string is capped at SQLite's default
  `SQLITE_MAX_LENGTH` (1e9, checked on the first value too) and the distinct set
  at 1M entries, each returning a `ResourceLimit` error rather than growing
  unbounded.

## [0.4.29] - Unreleased

### Fixed

- **Integer arithmetic overflow now promotes to REAL instead of erroring.** When
  the exact `i64` result of `+`, `-`, `*`, or unary `-` does not fit, SQLite
  redoes the operation in floating point and yields a REAL — it never errors or
  wraps. `9223372036854775807 + 1` = `9.2233720369e18` (real), `min_i64 - 1` and
  `max_i64 * 2` likewise, and `-(-9223372036854775808)` = `9.2233720369e18`.
  `checked_int_binop`'s int/int arm now falls back to `float_op` on the widened
  `f64` operands (the same float path the mixed-operand arms use), and the unary
  `Neg` arm falls back to `-(n as f64)` for `i64::MIN`; the `op_name` parameter,
  which existed only for the removed overflow-error message, was dropped.
  Non-overflowing arithmetic still returns INTEGER. (Division/modulo of
  `i64::MIN / -1` — where the operands are true integers rather than
  overflow-to-REAL literals — is a distinct edge left for a follow-up, tangled
  with SQLite's integer-coercing `%` semantics.)

## [0.4.28] - Unreleased

### Fixed

- **`NOT BETWEEN` now returns the logical negation of the range**, not a
  strict/exclusive-bounds test. `eval_between`'s non-plain branch computed
  `val > lo AND val < hi` (exclusive bounds), but codegen only ever emits
  `Between(false)` for `NOT BETWEEN`, whose meaning is `NOT(lo <= val <= hi)` =
  `val < lo OR val > hi`. The old code inverted the answer for interior values:
  `5 NOT BETWEEN 1 AND 10` wrongly returned `1` (5 IS in `[1,10]`, so the result
  is `0`) and `15 NOT BETWEEN 1 AND 10` wrongly returned `0`. Now it computes the
  inclusive range once and flips the boolean when negated; NULL operands still
  yield NULL. The `Between(bool)` payload is documented as `!negated` (true =
  `BETWEEN`, false = `NOT BETWEEN`). A pre-existing latent bug — no oracle case
  had exercised `NOT BETWEEN` before; two now do (in mini-sqlite). Also enables
  correct results for the new explicit-`COLLATE`-before-`BETWEEN` surface.

## [0.4.27] - Unreleased

### Fixed

- **Text/blob truthiness now takes numeric affinity.** `is_truthy` treated every
  non-NULL text/blob as true, so `WHERE <text-column>` kept all rows and `NOT
  'abc'` returned false. It now coerces via `cast_to_f64` and tests `!= 0`,
  matching SQLite: `NOT 'abc'` = 1, `NOT '5'` = 0, `'5' AND 1` = 1, `'abc' AND 1`
  = 0, and `WHERE s` / `CASE WHEN s` keep only numerically-non-zero text. This
  affects every boolean context uniformly (WHERE, HAVING, AND/OR, NOT, IIF,
  CASE WHEN) since they all route through `is_truthy`.

## [0.4.26] - Unreleased

### Fixed

- **Binary arithmetic now applies numeric affinity to text/blob operands.**
  `'5' + 0` errored ("cannot perform arithmetic on TEXT"); it now coerces via the
  shared `coerce_arith`/`text_to_numeric` path and evaluates to 5, matching
  SQLite. `'abc' + 1` = 1 (no numeric prefix → 0), `'10' - '3'` = 7, `'5' * 2` =
  10; division and modulo coerce too (`5 / '2'` = 2, `5 / '0'` = NULL, `'7' % 3`
  = 1). Bool operands use their integer value. Coercion is scoped to arithmetic
  only — comparison and bitwise operators keep their own rules. Known edge shared
  with unary minus: an integral real-syntax string (`'9.0'`) collapses to an
  integer, so `'9.0' / 2` is 4 not SQLite's 4.5 (float-affinity follow-up).

## [0.4.25] - Unreleased

### Fixed

- **`IN` now uses `=` equality and three-valued NULL logic.** The `InList`
  instruction tested membership with same-variant equality, so `1 IN (1.0)`
  wrongly returned false and a NULL list element was ignored. It now uses
  `sql_eq` (INTEGER/REAL compare numerically; text vs integer do not match) and
  follows SQLite's three-valued rule: a match → true (even alongside NULLs); no
  match with a NULL element present → NULL (`1 IN (NULL,2)`); otherwise false.
  `NOT IN` inherits this, so `5 NOT IN (NULL,2)` is NULL. Collation-wrapped
  operands (0.2.21's IN-collation) still fold correctly since both sides are
  canonicalised before the compare.

## [0.4.24] - Unreleased

### Fixed

- **`||` (concatenate) now treats a blob operand as its raw bytes.** `X'41' ||
  'B'` was producing `"x'41'B"` (the hex *display* form of the blob) instead of
  SQLite's `'AB'` (0x41 = 'A'). The `Concat` arm now stringifies a blob via its
  raw bytes (`concat_operand_to_str`, lossy-UTF-8) while `sql_to_str` keeps its
  reversible `x'…'` form for display everywhere else. Result is TEXT; NULL still
  propagates. blob||text, text||blob, and blob||blob all fold to the byte string.

## [0.4.23] - Unreleased

### Fixed

- **`LENGTH()` now accepts blobs and numbers.** It previously errored on
  anything but text/NULL; `LENGTH` now measures a blob by its raw byte count
  (`length(x'0102ff')` = 3, `length(x'')` = 0 — distinct from text's character
  count) and a number by its decimal-text length (`length(12345)` = 5,
  `length(-7)` = 2), matching SQLite. Blob literals (0.5.42 / this engine's
  front end) made the blob case reachable from SQL. Floats stay declined (their
  text form is subtle — same stance as `OCTET_LENGTH`/`HEX`/`QUOTE`).

## [0.4.22] - Unreleased

### Fixed

- **Unary minus now applies numeric affinity to a text/blob operand.** SQLite
  coerces the operand of `-` to a number before negating; `eval_unary`'s `Neg`
  arm previously left non-numeric values unchanged, so `-'5'` wrongly returned
  the string `'5'`. It now coerces through the shared `text_to_numeric` helper:
  `-'5'` = -5, `-'12abc'` = -12 (leading numeric prefix), `-'abc'` = 0 (no
  prefix), `-'3.5'` = -3.5, `-'  7'` = -7 (whitespace tolerated), `-TRUE` = -1.
  The `-i64::MIN` overflow guard is preserved. Known edge left for later: an
  exponent-form string (`'3e2'`) stays REAL in SQLite but collapses to an
  integer here.

## [0.4.21] - Unreleased

### Fixed

- `UPPER`/`LOWER` are now ASCII-only, matching SQLite's built-ins: only `a`–`z`/
  `A`–`Z` fold, and accented or non-Latin characters pass through unchanged
  (`upper('naïve')` → `'NAïVE'`, not `'NAÏVE'`). Previously they used Rust's
  full-Unicode `to_uppercase`/`to_lowercase`.

## [0.4.20] - Unreleased

### Added

- `Instruction::LikeEscape(negated)` implements `LIKE … ESCAPE` via
  `like_match_escape`: the escape character before a `%`, `_`, or itself makes
  that character a literal.

### Fixed

- **`NOT LIKE` was inverted** — the `negated` flag was dropped, so `NOT LIKE`
  behaved like `LIKE`. `Instruction::Like` now carries the flag and both LIKE
  instructions apply a NULL-aware inversion (`NULL` stays `NULL`;
  `matched ^ negated` otherwise).

## [0.4.19] - Unreleased

### Fixed

- **`substr()` edge cases now match SQLite.** `substr(X, 0)` treats `Y = 0` as a
  virtual slot before the first character (2-arg → the whole string; with a
  length it consumes one from `Z`), and a **negative length** returns the `|Z|`
  characters *preceding* the start, reading leftward — `substr('hello',2,-1)` is
  `'h'`, not `''`. Previously `Y = 0` returned `''` and a negative length
  clamped to `''`. Reimplemented as `sqlite_substr`, mirroring SQLite's
  `substrFunc` index arithmetic exactly (and still character-based, so multibyte
  UTF-8 counts by code point).

## [0.4.18] - Unreleased

### Added

- **`CAST(… AS NUMERIC)` runtime conversion** (`CastType::Numeric`). Applies
  SQLite's NUMERIC affinity: a number is unchanged (INTEGER stays INTEGER, REAL
  stays REAL — `CAST(3.0 AS NUMERIC)` is `3.0`, not `3`); text/blob is parsed and
  collapsed to INTEGER when the value is integral and fits `i64` (`'3.0'`→`3`,
  `'1e3'`→`1000`, `'42abc'`→`42`), otherwise REAL (`'3.5'`, an i64-overflowing
  integer like `'99999999999999999999'`→`1e20`); non-numeric text → `0`. The
  integer prefix is parsed exactly (not via f64) so `i64::MAX` round-trips as an
  integer. New helpers `cast_to_numeric` / `text_to_numeric`.

## [0.4.17] - Unreleased

### Changed

- **Division and modulo by zero now yield `NULL`, not an error.** `x / 0`,
  `x % 0`, `x / 0.0`, `x % 0.0`, and `0 / 0` return `SqlValue::Null` — matching
  SQLite, where a zero divisor is a NULL result rather than a runtime failure.
  Previously the VM raised `VmError::DivisionByZero`, which aborted the whole
  query (a statement that runs fine in SQLite would hard-fail in mini-sqlite).
  Applies to both integer and float zero divisors; the `Mod` arm also gained the
  missing float-zero check (`5.5 % 0.0` → `NULL` instead of `NaN`). NULL operands
  are still short-circuited to NULL upstream, and non-zero division/modulo is
  unchanged. `VmError::DivisionByZero` is retained but no longer constructed.

## [0.4.16] - Unreleased

### Added

- **Internal `__collate(value, name)` builtin.** Canonicalises a text value for
  the given collation (NOCASE → ASCII-lowercase, RTRIM → strip trailing spaces),
  passing NULL and every non-text value through unchanged, so a following byte
  comparison honours the collation without changing numeric semantics. Reuses the
  existing `collate_text` helper (from the ORDER BY COLLATE slice). Emitted by the
  planner for expr-level COLLATE; not user-facing SQL.

## [0.4.15] - Unreleased

### Added

- **Execute bitwise ops.** `&`/`|`/`~` coerce operands to integer (reusing the
  CAST slice's integer affinity), propagate NULL, and complement/AND/OR as i64.
  Shifts go through new `sql_shift`, which implements SQLite's exact rules —
  negative count flips direction, count ≥ 64 saturates (left → 0; right → 0/−1
  by sign), right shift is arithmetic — avoiding Rust's shift-overflow UB via
  `wrapping_shl` and width-checked branches.

## [0.4.14] - Unreleased

### Added

- **Collation-aware `ORDER BY` comparator.** `apply_sort` now compares text
  values through the sort key's collating sequence: `NOCASE` folds ASCII case
  on both operands, `RTRIM` strips trailing spaces, and `BINARY`/absent keeps
  raw byte order. Collation applies only to text-vs-text comparisons; every
  other type pairing uses the existing `sql_cmp` type ordering. The stable sort
  already preserves insertion order for equal keys, matching SQLite. New helpers
  `sql_cmp_collated` and `collate_text`.

## [0.4.13] - Unreleased

### Added

- **`NULLS FIRST`/`NULLS LAST` in the sort comparator.** `apply_sort` now places
  NULLs explicitly per the sort key's `nulls_first` (defaulting to `ascending`,
  which reproduces SQLite's default of NULLs-first-for-ASC / last-for-DESC).
  NULL placement is absolute and is not flipped by the ASC/DESC reversal, so an
  override like `ASC NULLS LAST` works.

## [0.4.12] - Unreleased

### Added

- **`Instruction::Cast` execution** (`apply_cast`) with SQLite's documented
  rules for the three supported target types: INTEGER (reals truncate toward
  zero; text yields its leading integer prefix), REAL (text yields its leading
  real prefix, exponent-aware), and TEXT (decimal string; a boolean renders as
  `1`/`0`). NULL always casts to NULL. Helpers `parse_int_prefix` /
  `parse_real_prefix` do the byte-scan prefix parsing.

## [0.4.11] - Unreleased

### Added

- **`PRINTF(format, …)` / `FORMAT(format, …)`** — C-style string formatting. A
  self-contained format engine (`sql_printf`) supports the conversions `%d`/`%i`,
  `%s` (with `.precision` truncation), `%x`/`%X`, `%o`, `%c` (first character of
  the argument), `%q` (single-quotes doubled), and `%%`; the flags `-` / `0` /
  `+` / space; and a field width. Missing arguments default to `0` / `""` and
  extra arguments are ignored, matching SQLite; a NULL format yields NULL.
  Float conversions (`%f`/`%g`/`%e`) are declined (their exact SQLite text form
  is the subtlety HEX/QUOTE avoid). **DoS-bounded:** field width/precision are
  capped at 1e6 and total output at 1e7, so a hostile `printf('%9999999999d')`
  is rejected rather than allocated. Arity is checked before indexing.

## [0.4.10] - Unreleased

### Added

- **`GLOB(pattern, subject)`** — the function form of SQLite's `GLOB` operator: a
  case-sensitive wildcard match returning `1` / `0` (`*` = any run, `?` = any
  single character, `[...]` = character class with `[^...]` negation and `a-c`
  ranges; a backslash is a literal, GLOB has no escape). NULL in either argument
  → NULL; arguments are matched by Unicode character. The matcher (`glob_match`)
  uses the same iterative two-pointer backtracking as `like_match`, so it is
  `O(text × pattern)` — no exponential blow-up on adversarial `*`-heavy patterns.
  (The infix `GLOB` operator remains a separate grammar-level feature; this is
  the callable function.)

## [0.4.9] - Unreleased

### Added

- **`LIKELY(x)` / `UNLIKELY(x)` / `LIKELIHOOD(x, p)`** — SQLite's query-planner
  hint functions. They bias the optimizer's row-count estimates but have no
  effect on the result: each returns its first argument unchanged (any type,
  including NULL). `LIKELIHOOD`'s second argument `p` is a probability the planner
  uses as a hint and must be a number in `[0.0, 1.0]` (validated; out-of-range or
  non-numeric is an error). Arity is checked before indexing.

## [0.4.8] - Unreleased

### Added

- **`OCTET_LENGTH(x)`** — the number of *bytes*, in contrast to `LENGTH`'s count
  of characters: `OCTET_LENGTH('héllo')` = 6 where `LENGTH('héllo')` = 5. Text is
  measured as its UTF-8 bytes, a blob as its raw byte count, and an
  integer/boolean as its decimal-text bytes (`OCTET_LENGTH(123)` = 3); NULL →
  NULL. Floats are declined (their byte length depends on SQLite's subtle float
  text form — same convention as HEX/QUOTE). Arity is checked before indexing.
  (The `LENGTH` doc is corrected: it counts characters, not bytes.)

## [0.4.7] - Unreleased

### Fixed

- **`HEX(NULL)`** now returns the empty string `''` (a text value), matching
  SQLite, instead of NULL. SQLite casts HEX's argument to a blob first, so
  `NULL` becomes an empty blob and hexing it yields `''` (`typeof` is `text`).
  Surfaced while adding UNHEX (`HEX(UNHEX('abc'))` should be `''`, not NULL).

## [0.4.6] - Unreleased

### Added

- **`UNHEX(x)` / `UNHEX(x, ignore)`** — the inverse of `HEX`: decode a string of
  hexadecimal digit pairs into a blob (case-insensitive). `unhex('414243')` →
  `x'414243'`, `unhex('')` → empty blob; an odd number of digits or a non-hex
  character yields NULL. The optional second argument is a set of ignorable
  characters, which SQLite permits only at a byte boundary — never splitting a
  pair: `unhex('41.42', '.')` → `x'4142'` but `unhex('4-1-4-2', '-')` → NULL.
  Integer/boolean arguments coerce to their decimal text; NULL in either argument
  → NULL. Output is bounded by the input length (no unbounded allocation), and
  arity is checked before indexing.

## [0.4.5] - Unreleased

### Fixed

- **`ROUND(x, n)` with a negative digit count** now matches SQLite, which treats
  a negative `n` as zero rather than rounding to tens/hundreds:
  `ROUND(2.567, -1)` = `ROUND(2.567, 0)` = `3.0` (was `0.0`), and
  `ROUND(12.5, -1)` = `13.0`. Positive/zero digit counts, round-half-away-from-
  zero, and NULL propagation are unchanged.

## [0.4.4] - Unreleased

### Added

- **`CONCAT(x, …)`** — concatenate every argument's text. A NULL argument
  contributes the empty string (it does not nullify), so
  `CONCAT('a', NULL, 'c')` = `'ac'`; the result is always text; at least one
  argument is required.
- **`CONCAT_WS(sep, x, …)`** — join the value arguments with `sep`. Unlike
  CONCAT, a NULL value argument is *skipped* entirely (`CONCAT_WS('-','a',NULL,'c')`
  = `'a-c'`); a NULL separator makes the whole result NULL; at least two
  arguments are required.
- **`SUBSTRING`** — an accepted spelling of the existing `SUBSTR`.

  Integer/boolean arguments coerce to their decimal text; Float/Blob arguments
  are declined (their SQLite text form is subtle — same convention as HEX/QUOTE).

## [0.4.3] - Unreleased

### Added

- **Two-argument `TRIM(x, y)` / `LTRIM(x, y)` / `RTRIM(x, y)`** — the second
  argument is a *set of characters* stripped from both / the left / the right
  end, matching SQLite: `TRIM('xxhixx', 'x')` → `'hi'`,
  `TRIM('abcHIcba', 'abc')` → `'HI'`. Trimming operates on Unicode characters,
  not bytes (`TRIM('héllo', 'h')` → `'éllo'`); an empty set removes nothing; a
  NULL in either argument propagates to NULL; integer/boolean arguments coerce
  to their decimal text (`TRIM(12321, '1')` → `'232'`). The single-argument
  whitespace forms are unchanged. The three arms now share one `trim_builtin`
  helper. The helper validates arity before indexing (so `TRIM()` — which the
  grammar permits — is a clean error, not an out-of-bounds panic) and resolves
  the trim-set through a `HashSet` so the operation stays `O(N + M)` rather than
  `O(N·M)` in the subject / set lengths.

### Added

- **Scalar `MAX(a, b, …)` / `MIN(a, b, …)`** (two-or-more arguments): return the
  largest / smallest argument, or NULL if any argument is NULL, comparing with
  SQL value order. The single-argument aggregate forms are unchanged (compiled to
  `FinalizeAgg`); the planner (sql-planner 0.2.0) now routes only the multi-arg
  calls here. Fixes `SELECT MAX(3, 9, 5)` returning `3` instead of `9`.

## [0.4.1] - Unreleased

### Added

- **`IIF(x, y, z)`** — SQLite's function-form conditional, equivalent to
  `CASE WHEN x THEN y ELSE z END`: returns `y` when `x` is truthy (SQL
  three-valued logic — a NULL or falsy `x` selects `z`), reusing the engine's
  `is_truthy` helper. Unit-tested and validated against real SQLite by the
  mini-sqlite differential oracle. (A pure-VM partial for `CASE`, which is
  blocked on a stale generated grammar — see the mini-sqlite notes.)

## [0.4.0] - Unreleased

### Added

- **Five more scalar built-in functions** in `call_builtin`, matching SQLite:
  - `SIGN(x)` — `-1`/`0`/`+1` for a negative/zero/positive number; NULL for a
    NULL or non-numeric argument.
  - `UNICODE(s)` — the code point of the first character of `s`; NULL for a NULL
    or empty string.
  - `CHAR(x1, …)` — a string built from the argument code points (out-of-range
    or non-integer arguments contribute nothing; no args → `""`).
  - `ZEROBLOB(n)` — a BLOB of `n` zero bytes (`n < 0` → empty); NULL → NULL. The
    length is capped at 1,000,000 (returning `ResourceLimit`, like the GROUP BY /
    COUNT(DISTINCT) guards) so a query such as `zeroblob(9999999999)` can't force
    a multi-gigabyte eager allocation.
  - `QUOTE(x)` — the value as an SQL literal (NULL → `NULL`, text single-quoted
    with doubled inner quotes, blob as `X'…'` hex, integer as its digits; floats
    declined, like `HEX`).
  Each is unit-tested and validated end-to-end against real SQLite by the
  mini-sqlite differential oracle.

## [0.3.0] - Unreleased

### Added

- **Five scalar built-in functions** in `call_builtin`, matching SQLite:
  - `IFNULL(a, b)` — the two-argument `COALESCE`.
  - `NULLIF(a, b)` — NULL when the arguments are equal, else `a`.
  - `TYPEOF(x)` — the storage-class name (`null`/`integer`/`real`/`text`/`blob`).
  - `INSTR(haystack, needle)` — 1-based **character** index of the first match
    (0 if absent, NULL on a NULL argument, 1 for an empty needle).
  - `HEX(x)` — uppercase hex of the argument's bytes (text → UTF-8, blob → raw,
    integer → decimal-text bytes; NULL → NULL; floats are declined).
  These parsed as function calls already but hit the `unknown built-in function`
  fallthrough; each is now implemented and unit-tested, and validated end-to-end
  against real SQLite by the mini-sqlite differential oracle.

## [0.2.1] - Unreleased

### Fixed

- **Same-named output columns no longer collide.** Phase-4 materialization used
  to collapse each row's positional `(name, value)` pairs into a
  `HashMap<String, SqlValue>` and then re-read one value per output-column name.
  Two output columns that share a name — e.g. `SELECT UPPER(x), LENGTH(x)` (both
  default to `?`) or `SELECT id, id` — collided in the map, so every such column
  returned the *last* value (`SELECT UPPER(x), LENGTH(x)` came back as
  `LENGTH(x), LENGTH(x)`). The row buffer is already positional and parallel to
  the locked `output_columns` (both are produced by the same `EmitColumn`
  sequence, and hidden sort-key columns are truncated off both together), so we
  now project by **position** and drop the name-keyed map entirely. Fixes the
  differential-oracle `string_functions` divergence.

### Added

- **Outer-join match flag** — three instructions with no value-stack effect that
  let `sql-codegen` implement `LEFT`/`RIGHT OUTER JOIN`: `ClearMatch` (reset at
  the start of each outer row), `SetMatch` (an inner row satisfied `ON`), and
  `JumpIfMatched` (skip the NULL-padded emit when the outer row matched). The VM
  keeps a single `join_matched` boolean; a false condition still advances the
  loop, so termination and stack balance are unchanged.

## [0.1.0] — 2026-07-01

### Added

- Initial implementation of the Mini-SQLite Level 1 stack-machine VM.
- `execute(program, backend)` public entry point returns `QueryResult`.
- `QueryResult` struct with `columns`, `rows`, and `rows_affected` fields.
- `VmError` enum covering StackUnderflow, CursorNotFound, LabelNotFound,
  TypeMismatch, DivisionByZero, AggIndexOutOfRange, BackendError.
- Full instruction set support:
  - Stack: `LoadConst`, `LoadColumn`
  - Arithmetic: `BinaryOpInstr` (Add, Sub, Mul, Div, Mod)
  - Comparison: `BinaryOpInstr` (Eq, Neq, Lt, Lte, Gt, Gte)
  - Logic: `BinaryOpInstr` (And, Or) with SQL three-valued / Kleene logic
  - String: `BinaryOpInstr` (Concat)
  - Unary: `UnaryOpInstr` (Neg, Not) with NULL propagation
  - NULL tests: `IsNull`, `IsNotNull`
  - Pattern: `Like` (iterative NFA, no regex, ReDoS-safe)
  - Range: `Between` (inclusive and exclusive bounds)
  - Membership: `InList`
  - Scan: `OpenScan`, `AdvanceCursor`, `JumpIfExhausted`, `CloseScan`
  - Row assembly: `BeginRow`, `EmitColumn`, `EmitRow`
  - Aggregation: `InitAgg`, `UpdateAgg`, `FinalizeAgg`, `SaveGroupKey` (no-op)
  - Control flow: `Label`, `Jump`, `JumpIfTrue`, `JumpIfFalse`, `Halt`
  - DDL: `CreateTableInstr`, `DropTableInstr`
  - DML: `InsertRow` (functional), `UpdateRows` (Level 1 stub), `DeleteRows` (Level 1 stub)
  - Transactions: `BeginTransaction`, `CommitTransaction`, `RollbackTransaction`
  - Post-ops: `SortResult`, `DistinctResult`, `LimitResult`
- Eager cursor buffering: all rows fetched at `OpenScan` time.
- Exhaustion-flag cursor model: `AdvanceCursor` sets `exhausted` flag;
  `JumpIfExhausted` reads the flag so the last row is consumed before jumping.
- Post-op pass: a second instruction-scan after `Halt` collects SortResult /
  DistinctResult / LimitResult so they apply to the final result buffer.
- Literate programming style: all functions include inline explanations,
  truth tables, diagrams, and examples.
- 75 unit tests covering all instruction groups, edge cases, and error paths.
- BUILD (bash) and BUILD_windows (PowerShell) scripts.

### Known limitations (Level 1)

- `UpdateRows` counts rows affected but does not persistently update the
  backend.  The `Backend::update()` API requires a `Cursor` keyed to the
  table, which can only be constructed via `InMemoryBackend::open_cursor`
  (a non-trait method).  Level 2 will close this gap.
- `DeleteRows` removes rows from the local cursor buffer (preventing re-visits)
  but does not call `Backend::delete()` for the same reason.
- GROUP BY aggregation uses a single global accumulator; per-group aggregation
  requires a hash-map grouping strategy (Level 2).
