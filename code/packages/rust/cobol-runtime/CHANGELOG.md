# Changelog

## 0.31.0 — cross-category MOVE (unsigned-integer numeric → alphanumeric)

- The first **cross-category** `MOVE`: `MOVE numeric-item TO alphanumeric-item`,
  restricted to an **unsigned integer** source (`PIC 9(n)` — no `S`, no `V`) into a
  `PIC X(m)` receiver. Oracle-first and byte-identical to `cobol-iir-compiler`
  0.27.0. No grammar change.
- **The rule.** The numeric sending item is treated as though it held its **digit
  characters** — its `n`-digit zero-padded magnitude, exactly what `DISPLAY` shows
  — then moved by the alphanumeric rules (LEFT-justified, space-padded on the right
  if wider, truncated on the right if narrower). `move_into` already did this: for a
  `Src::Num` into an alphanumeric picture it takes `Decimal::digits()` (the
  `int`+`frac` characters — for an unsigned integer, exactly the `n` zero-padded
  digits) and stores it through the existing `move_into_char` left-justify/pad path.
  So `PIC 9(3)` holding `42` → `X(3)` is `"042"`, → `X(5)` is `"042  "`, → `X(2)`
  is `"04"`.
- **Deferral gate (`exec_move`).** Only an unsigned integer source is supported on
  this rung. A **signed** (`PIC S9`) or **scaled** (`PIC 9V9`) numeric source into
  an alphanumeric receiver is now a clean `Unsupported` reject, mirroring the
  compiler's compile-time reject of the same shapes so the two engines agree on the
  deferral (not just on the accepted case). The **reverse** direction (alphanumeric
  → numeric) was and remains rejected in `move_into`, and a group item stays a
  later rung.
- **Tests.** Oracle-unit cases for the three supported shapes (exact-fit, pad,
  truncate) and each deferred reject (signed source, scaled source, alpha →
  numeric).

## 0.30.0 — computed (data-name) reference modification

- Generalised `Operand::RefMod` to carry `start`/`len` as a new `RefIndex`
  (`Lit(usize)` **or** `Name(String)`) instead of raw `usize`. `read_refmod_index`
  now accepts a bare data-name index (`RefIndex::Name`) alongside an integer
  literal (`RefIndex::Lit`); a signed/fractional literal or nested reference
  modification as the index is a clean later-rung reject. No grammar change.
- `refmod_string` evaluates the (possibly computed) `start`/`len` to `i64` via the
  new `refmod_index_value` — a literal is its own value; a data-name must be an
  **unsigned-integer** item (`PIC 9…`, no `S`, no `V`), its stored digits parsed —
  then computes `start0 = start - 1` and `end = start0 + len` (or `end = width` for
  an omitted length) and slices `[start0, end)`.
- **Out-of-range rule.** A computed reference modification returns the new
  `RuntimeError::RefModOutOfRange` trap exactly when
  `start0 < 0 || end < start0 || end > width` — the *identical* predicate the
  compiled `str_slice` enforces in the VM/wasm backends
  (`start < 0 || end < start || end > s.len()`). So an in-range computed refmod
  slices byte-identically to `cobol-iir-compiler` 0.26.0, and an out-of-range one
  errors on both engines rather than producing a silently wrong slice.
- Added `RuntimeError::RefModOutOfRange(String)` — a genuine run-time bounds trap,
  distinct from `Unsupported` (an unmodelled feature).
- Deferred (unchanged): a signed/fractional/non-numeric index item, reference
  modification of a numeric item, and use in a numeric/`MOVE`-source context.
- Tests: 7 new oracle unit tests — computed mid-substring, omitted length,
  out-of-range and zero-start traps, numeric base reject, signed index reject, and
  MOVE-source reject.

## 0.29.0 — `INSPECT … CONVERTING from TO to`

- Added `Stmt::InspectConverting { source, from, to }` and its executor
  `exec_inspect_converting`. `INSPECT source CONVERTING from TO to` translates each
  character of the alphanumeric `source` through a per-character **translation
  table** built from the two EQUAL-length string literals `from` and `to`.
- Semantics (oracle = source of truth): a source character equal to `from[k]`
  becomes `to[k]`; if `from` repeats a character the **FIRST (leftmost) entry
  wins** (the map is built with `or_insert`, which never overwrites); a character
  in no table entry is left unchanged. The map preserves length, so the rebuilt
  string feeds the same alphanumeric char-store path a `MOVE` uses. Example:
  `CONVERTING "AEIOU" TO "12345"` on `BEAN` → `B21N`; `CONVERTING "AAB" TO "XYZ"`
  on `AAB` → `XXZ` (A→X wins over the later A→Y).
- Reader: `read_inspect_converting` extracts the two string-literal operands and
  rejects a `BEFORE`/`AFTER` region; `read_converting_literal` rejects a
  data-name / figurative / numeric-literal / reference-modified `from`/`to`. The
  unequal-length check lives in the executor (a clean later-rung `Unsupported`).
- `CONVERTING` is a **standalone** `INSPECT` alternative — combining it with a
  `TALLYING`/`REPLACING` clause in one statement does not parse (a
  `RuntimeError::Parse`), never a mis-run. A numeric/group source is rejected by
  the shared `inspect_alnum_source`.
- Later rungs: unequal-length `FROM`/`TO`, a `PIC X` item / figurative /
  reference-modified `from`/`to`, and a `BEFORE`/`AFTER` region.

## 0.28.0 — combined `INSPECT … TALLYING … REPLACING` (one statement)

- Added `Stmt::InspectTallyReplace { source, counter, delim, search, replace }`
  and its executor `exec_inspect_tally_replace`. One `INSPECT` carrying BOTH
  phrases: `INSPECT source TALLYING counter FOR ALL delim REPLACING ALL x BY y`.
- Semantics (oracle = source of truth): per ISO the statement runs "as though an
  `INSPECT TALLYING` were specified, followed by an `INSPECT REPLACING`". So the
  order is fixed — count `delim` in the **ORIGINAL** source and ADD to `counter`
  FIRST, THEN replace every `x` with `y`. When `delim == x` this ordering is
  observable: the tally must count every pre-replacement occurrence (e.g.
  `MISSISSIPPI` TALLYING `S` gives 4, then `S`→`Z`), which running the count after
  the replace would miss.
- Refactor: the source alphanumeric check, the TALLYING count-and-add, and the
  REPLACING map were factored out of `exec_inspect`/`exec_inspect_replacing` into
  `inspect_alnum_source`, `inspect_tally`, and `inspect_replace` so all three
  execs (lone TALLYING, lone REPLACING, combined) share the exact same logic and
  the combined case composes them in order — tally on the current storage, then
  replace in place.
- The `inspect_stmt` reader now matches on `(has_tally, has_repl)`: the combined
  case extracts both phrases (each rejecting its own later-rung forms) and builds
  the new variant; the two `has_tally && has_repl` rejects (reader and exec) are
  gone. No grammar/lexer/parser change was needed — the grammar already accepted
  `inspect_tallying [inspect_replacing]`.
- Later rungs (clean `RuntimeError::Unsupported`): a combined statement whose
  TALLYING half is `LEADING`/`CHARACTERS`/several counters or FOR phrases/a
  region, or whose REPLACING half is `CHARACTERS`/`LEADING`/`FIRST`/several
  items/a region — the combined gate does not admit the deferred sub-forms.
- Tests: the former "combined is a later rung" assertion became a positive check
  (combined counts then replaces), plus a shared-char ordering test
  (`MISSISSIPPI`, `S` counted then replaced) and a still-deferred combined
  (`FOR LEADING … REPLACING …`) reject.

## 0.27.0 — `INSPECT … REPLACING ALL … BY …` (first rung)

- Added `Stmt::InspectReplacing { source, search, replace }` and its executor
  `exec_inspect_replacing`. `INSPECT source REPLACING ALL x BY y` replaces EVERY
  occurrence of the SINGLE character `x` (a 1-character string literal or a `PIC
  X(1)` item) in the alphanumeric `source` with the SINGLE character `y`, **in
  place**.
- Semantics (oracle = source of truth): both operands are single characters, so
  the source's width is unchanged — a straight **per-position map**
  (`c == x ? y : c`), left to right. The rebuilt string is stored back through the
  same alphanumeric char-store path (`move_into` with `Src::Chars`) a `MOVE` uses,
  so the compiled `cobol-iir-compiler` unroll matches this reference byte-for-byte.
  Both `x` and `y` are validated by the shared `single_delim_char`, and both are
  read before mutating so an invalid replacement leaves the source untouched.
- The `inspect_stmt` reader now shares its source parsing and dispatches on
  whether a `TALLYING`, a `REPLACING`, or both clauses are present; the combined
  `TALLYING … REPLACING` in one `INSPECT` is a clean later rung.
- Later rungs (clean `RuntimeError::Unsupported`): `REPLACING CHARACTERS BY`,
  `REPLACING LEADING`/`FIRST`, `BEFORE`/`AFTER` regions, several replace items, the
  combined `TALLYING … REPLACING`, a multi-character / figurative / numeric /
  wider-than-one search or replacement, and a numeric/group source.
- Tests: a positive executor test (a repeated char mapped, an absent char leaving
  the source unchanged, and `PIC X(1)` search/replacement items) plus a later-rung
  test covering `CHARACTERS`, `LEADING`, a multi-character search, several replace
  items, and the combined `TALLYING … REPLACING`.

## 0.26.0 — `INSPECT … TALLYING … FOR ALL` (first rung)

- Added `Stmt::Inspect { source, counter, delim }` and its executor.
  `INSPECT source TALLYING counter FOR ALL delim` counts the (non-overlapping,
  left-to-right) occurrences of the SINGLE-character `delim` (a 1-character string
  literal or a `PIC X(1)` item) in the alphanumeric `source`, then **ADDs** that
  count to the counter.
- Semantics (oracle = source of truth): INSPECT **adds** to the counter — it does
  NOT clear it first — so the net effect is `counter := counter + occurrences`.
  The count folds in through the same `store_result` path the arithmetic verbs use
  (COBOL's silent high-order truncation on overflow), so the compiled
  `cobol-iir-compiler` scan loop matches this reference byte-for-byte. The counter
  must be an unsigned integer numeric item (`PIC 9(n)`).
- Later rungs (clean `RuntimeError::Unsupported`): `FOR LEADING` / `FOR
  CHARACTERS` tallies, `BEFORE`/`AFTER` regions, several `TALLYING` counters or
  `FOR` phrases, any `REPLACING` (`INSPECT … REPLACING` and `INSPECT … TALLYING …
  REPLACING`), a multi-character / figurative / numeric / wider-than-one delimiter,
  and a numeric/group source or a non-integer/signed/non-numeric counter.

## 0.25.0 — `UNSTRING … DELIMITED BY … INTO` (first rung)

- Added `Stmt::Unstring { source, delim, targets }` and its executor — the inverse
  of `STRING`. `UNSTRING source DELIMITED BY delim INTO r1 [r2 …]` scans the
  alphanumeric `source` left to right and splits it into delimited fields on each
  occurrence of the SINGLE-character `delim` (a 1-character string literal or a
  `PIC X(1)` item), moving successive fields into successive receivers as ordinary
  alphanumeric `MOVE`s (left-justified, space-padded, truncated — reusing
  `move_into`).
- Semantics (oracle = source of truth): each receiver **including the last** takes
  the field up to the NEXT delimiter (or end-of-source); extra fields beyond the
  receiver count are dropped (that would be `ON OVERFLOW`, a later rung); an empty
  field from consecutive or leading delimiters yields all spaces; and once the
  source is exhausted (a field ran to end-of-source with no trailing delimiter) the
  remaining receivers are **left unchanged** — not space-filled.
- Later rungs (clean `RuntimeError::Unsupported`): `WITH POINTER`, `ON`/`NOT ON
  OVERFLOW`, a multi-character / `ALL` / `OR` delimiter, a numeric or figurative
  delimiter, and a numeric/group source or receiver.

## 0.24.0 — `STRING … DELIMITED BY SIZE INTO` (first rung)

- Added `Stmt::String { sources, target }` and its executor. `STRING s… DELIMITED
  BY SIZE INTO t` concatenates every sending field — each taken in FULL
  (`DELIMITED BY SIZE`), so a `PIC X(5)` carries its trailing spaces — left to
  right, then overlays the result onto the alphanumeric receiver `t` from the
  left. Following the ANSI-85 rule, STRING writes only the characters it produced
  and **leaves the rest of `t` unchanged** (no space-fill, unlike `MOVE`),
  truncating at `t`'s width when the result is longer.
- Sending fields this rung: alphanumeric items and string / numeric literals (a
  numeric literal contributes its source digits verbatim). A numeric item, a group
  item, and a figurative constant as a sending field are clean
  `RuntimeError::Unsupported` "later rung" errors, as are a real
  (identifier/literal) delimiter, `WITH POINTER`, and `ON`/`NOT ON OVERFLOW`, and a
  non-alphanumeric receiver.

## 0.23.0 — reference modification `IDENT(start:len)`

- The oracle now evaluates COBOL **reference modification** — `base(start:len)`
  selects `len` characters of alphanumeric item `base` from 1-based position
  `start`; `base(start:)` (omitted length) runs to the end of the item.
  `Operand::RefMod { base, start, len }` is added to the `Operand` enum and
  constructed in `read_operand` when the parser's reference-modification suffix
  is present (both `start` and `len` must be integer NUMBER literals on this
  rung; a computed start/length is a later rung). `src_from_operand` and
  `display_image` gain a `RefMod` case that slices the base item's characters
  `[start-1, start-1+len)` via a new `refmod_string` helper, so DISPLAY prints
  the substring and alphanumeric comparisons compare it — byte-identical to the
  `cobol-iir-compiler`'s constant-index `str_slice`. Reference modification of a
  numeric item, or as a MOVE source / in a numeric context, is a
  `RuntimeError::Unsupported` "later rung".

## 0.22.0 — alphanumeric `EVALUATE` subject

- `EVALUATE` now works over an **alphanumeric** subject (`EVALUATE GRADE WHEN "A"
  … WHEN "A" THRU "M" …`), not just numeric. The numeric-or-alphanumeric ordering
  used by relational conditions is factored out of `eval_relation` into
  `compare_operands(a, b) -> Ordering` (numeric `cmp_value` when both are numeric,
  else the space-padded character compare), and `exec_evaluate`/`subject_in_when`
  now use it — so a `WHEN` value matches by equality and a `THRU` range by
  `subject >= lo && subject <= hi` for character subjects too. No behavioural change
  to relations.

## 0.21.0 — `EVALUATE` multiple values and `THRU` ranges per `WHEN`

- A `WHEN` now carries a value-*list*: `Stmt::Evaluate`'s branch `when` becomes
  `Option<Vec<WhenValue>>` (`None` = `WHEN OTHER`), with `WhenValue::Single(Operand)`
  / `WhenValue::Range(Operand, Operand)`. A branch matches when the subject equals
  any single value or falls within any inclusive `THRU` range
  (`WHEN 1 5 THRU 7 9`). Each side is an `Operand` (literal or data-name), evaluated
  at match time. First matching branch runs (no fall-through); still iterative over
  both branches and the values within a branch (no recursion).

## 0.20.0 — `EVALUATE` (case statement)

- `Stmt::Evaluate { subject, branches }` — COBOL's case statement. Each branch's
  `when` is `Some(value)` or `None` (`WHEN OTHER`). `exec_evaluate` compares the
  subject to each value top-to-bottom and runs the **first** match's statements
  (`WHEN OTHER` matches once reached), with no fall-through; the branch's `Flow`
  propagates so a `STOP RUN`/`GO TO` inside a `WHEN` unwinds, like an `IF` branch.
  Branches are tested by **iteration**, so thousands of `WHEN`s cannot overflow the
  stack (covered by a regression test). Numeric subject/value this rung; an
  alphanumeric one is a later rung.

## 0.19.0 — `NOT` over a condition

- `Cond` gains `Not(Box<Cond>)`. The new `negation = [ "NOT" ] simple_condition`
  grammar layer reads a leading `NOT` (`read_negation`) and wraps the simple
  condition in `Cond::Not`; `eval_cond` returns `!eval_cond(inner)`. `NOT` binds
  tighter than `AND`/`OR` and works over a relation, a condition-name, or a
  parenthesised group (de Morgan, etc.).

## 0.18.0 — compound conditions (`AND` / `OR` / parentheses)

- `Cond` gains `And(Vec<Cond>)` and `Or(Vec<Cond>)`. `read_condition` reads a
  `disjunction` of `AND`-joined simple conditions (relation / condition-name /
  parenthesised), with `AND` binding tighter than `OR`. `eval_cond` short-circuits
  (`all` / `any`). `IF` and `PERFORM … UNTIL` accept compound conditions.
- The `AND`/`OR` parts are held as a **flat list**, not a nested binary tree, so a
  long chain (`A AND A AND …`, which is grammar *repetition* and so is not bounded
  by the parser's rule-depth cap) is evaluated by **iteration**. Recursion happens
  only into parenthesised groups, whose depth the parser does cap — so a crafted
  chain of thousands of terms cannot overflow the stack (covered by a regression
  test).

## 0.17.0 — symbolic relational operators

- `IF` / `PERFORM … UNTIL` conditions accept the symbols `>` `<` `=` `>=` `<=`
  `<>` as well as the word forms. `read_condition` now maps each operator to a
  base relation plus a *baseline* negation (`>=` ≡ "not <", `<=` ≡ "not >", `<>` ≡
  "not ="); a written `NOT` composes with that baseline by XOR. No change to
  `Cond` — the symbols reduce onto the existing `RelOp` + `negated` model.

## 0.16.0 — `SET cond-name TO TRUE`

- `Stmt::SetTrue { cond_name }` assigns a level-88 condition-name's conditional
  variable the value that makes it hold: the **first** of its `VALUE` items (the
  low bound of a leading `THRU` range). Numeric variable only, matching the
  condition-name test path; an alphanumeric conditional variable is a later rung,
  and an undeclared condition-name is an `UndefinedName` error.

## 0.15.0 — level-88 multiple values and `THRU` ranges

- A `VALUE` clause now parses into a `Vec<ValueSpec>` (`Single(Lit)` |
  `Range(Lit, Lit)`); `DataDef.value: Option<Lit>` becomes `DataDef.values`. A
  level-88 condition-name holds when its conditional variable equals **any** single
  value or falls within **any** inclusive `THRU` range
  (`88 OK VALUE 1 5 THRU 7 9`). A plain item must still carry exactly one single
  value — a multi-value or range `VALUE` on a non-88 item is a clean `Unsupported`
  error. Still numeric-variable-only; alphanumeric conditional variables remain a
  later rung.

## 0.14.0 — level-88 condition-names

- A `88 NAME VALUE lit.` entry now registers a boolean condition-name bound to the
  most recent item (its conditional variable), instead of being rejected as a
  deferred level. `Cond` becomes an enum — `Relation { … }` or
  `ConditionName(String)` — and `IF IS-OK` / `PERFORM … UNTIL IS-OK` evaluate the
  name as "does the variable equal the value?". This rung compares a **numeric**
  variable against a numeric value; an alphanumeric conditional variable, multiple
  values, and `THRU` ranges are clean `Unsupported` later rungs. Level-88 takes no
  storage, so the item-tree depth bound (≤ 49) is unchanged.

## 0.13.0 — re-export the PICTURE / value building blocks

- **Public re-exports:** `Picture`, `Decimal`, `move_into_char`, and
  `move_into_numeric` are now part of the crate's public API. This lets a
  *compiler* — `cobol-iir-compiler` (PL09 step 4) — reuse COBOL's exact
  picture and fixed-point-value logic to format literals into their stored
  picture image, so its compiled output is byte-identical to this interpreter's
  `DISPLAY`. No behavioural change to the interpreter itself.

## 0.12.0 — ROUNDED / ON SIZE ERROR on the arithmetic verbs

- **`ADD`/`SUBTRACT`/`MULTIPLY`/`DIVIDE` now take `ROUNDED` and `ON SIZE ERROR`**,
  matching `COMPUTE`. `ROUNDED` rounds half away from zero into the receiver
  (else truncate); `ON SIZE ERROR` runs its statements (receiver unchanged) when
  the result's integer part overflows — or, for `DIVIDE`, when the divisor is
  zero. Without a handler, overflow truncates silently and a zero divisor stays a
  hard `DivideByZero`.
- The store path (round → size-error → store) is now a shared `store_result`
  helper used by all five arithmetic verbs, so their rounding/overflow behaviour
  is identical. `DIVIDE` now computes at the same intermediate precision as
  `COMPUTE` before rounding into the receiver.

## 0.11.0 — PERFORM … THRU (paragraph range)

- **`PERFORM para-1 THRU para-2`** runs the whole range of paragraphs from
  `para-1` through `para-2` in source order (falling through between them), then
  returns. It composes with every repeat mode — `PERFORM A THRU B 3 TIMES`,
  `… UNTIL …`, `… VARYING …` all repeat the whole range.
- The grammar already parsed `THRU`/`THROUGH`; this wires up the runtime (the
  reader previously rejected it). A backwards range (`para-2` before `para-1`) is
  a clean error.
- The inline form and non-consecutive/`EXIT`-terminated ranges remain deferred.

## 0.10.0 — PERFORM … VARYING (counted loop)

- **`PERFORM para VARYING id FROM start BY step UNTIL cond`** sets the induction
  variable `id` to `start`, then runs the paragraph while `cond` is false
  (test-before), stepping `id` by `step` after each iteration.
- The `PERFORM` repeat forms are now modelled as a `PerformMode` enum
  (`Once` / `Times` / `Until` / `Varying`) instead of ad-hoc option fields — a
  cleaner substrate as the family grows.
- Iterative like the other loops (a never-satisfied `VARYING` hangs but never
  overflows the stack); a step overflow is a clean error; `STOP RUN` / `GO TO`
  in the body propagate.
- `WITH TEST AFTER`, multiple `AFTER` phrases, and `PERFORM … THRU` remain
  deferred.

## 0.9.0 — PERFORM … UNTIL (conditional loop)

- **`PERFORM para UNTIL cond`** repeats a paragraph while the condition is false,
  testing it **before** each iteration (so an initially-true condition runs the
  paragraph zero times) — COBOL's default `WITH TEST BEFORE`.
- The repeat loop is iterative, so even a never-satisfied `UNTIL` (an infinite
  loop — the programmer's bug, valid COBOL) does not grow the native stack. A
  `STOP RUN` / `GO TO` inside the body propagates out as its `Flow`.
- `PERFORM … VARYING` / `… THRU` / `WITH TEST AFTER` and the inline form remain
  deferred.

## 0.8.0 — GO TO (unconditional transfer) + program-counter execution

- **`GO TO para`** transfers control unconditionally to a paragraph. The
  procedure division now runs as a **program counter** over paragraphs: after a
  paragraph, control falls through to the next unless a `GO TO` jumped the counter
  or `STOP RUN` ended the program.
- The statement control signal changed from a stop-`bool` to a `Flow`
  (`Normal` / `Stop` / `GoTo(idx)`) that unwinds out of enclosing
  `IF`/`PERFORM`/`ON SIZE ERROR` up to the top-level loop.
- **`GO TO` back-edges form loops** (`IF … GO TO LOOP`) — driven iteratively by
  the program counter, so a loop never grows the native stack.
- A `GO TO` inside a performed paragraph transfers control at the top level
  (abandoning the `PERFORM`'s return) — the honest reading of "GO TO out of a
  range". `GO TO … DEPENDING ON`, `ALTER`, and range-return niceties are deferred.

## 0.7.0 — PERFORM (out-of-line paragraph invocation)

- **`PERFORM para [n TIMES]`** runs a named paragraph out of line and returns to
  the statement after the `PERFORM`. The `Machine` now indexes paragraphs by name
  and executes them by cloning their statement list (so a performed paragraph and
  the top-level fall-through share one execution path).
- The `TIMES` count is a value: `≤ 0` runs the paragraph zero times (COBOL's
  rule), a fractional count truncates, and an absurd (non-`usize`) count is a
  clean error.
- **`STOP RUN`** inside a performed paragraph ends the whole program (the
  stop-flag propagates out of the `PERFORM`).
- **Recursion guard:** a paragraph that performs itself (directly or in a cycle)
  is bounded by `MAX_PERFORM_DEPTH` (100) and fails with a clean error instead of
  overflowing the native stack.
- Deferred to later PRs: `PERFORM … THRU`, `… UNTIL`, `… VARYING`, the inline
  form, and `GO TO`.

## 0.6.0 — signed numerics (PIC S9…)

- **`PIC S9…` signed numeric fields.** The leading `S` marks the field signed and
  bears no storage position (`S9(4)` is still 4 digits). `picture.rs` parses it
  (and rejects a misplaced `S`, or `S` on a non-numeric field); the operational
  sign is carried alongside the magnitude digits on each item.
- **Sign is preserved** through `MOVE` and arithmetic into a signed receiver; an
  unsigned receiver still drops the sign to magnitude (unchanged). Zero is always
  unsigned.
- **`DISPLAY` overpunch.** A signed field displays its sign as a trailing
  ("zoned decimal") overpunch on the units digit under the default
  `SIGN IS TRAILING`: `+123` → `12C`, `−123` → `12L`, `0` → `00{`. This is the
  authentic COBOL rendering of a `DISPLAY`-usage signed field.
- Deferred to a later PR: the explicit `SIGN` clause and its `SEPARATE` /
  `LEADING` variants (this PR is the default trailing-overpunch sign only).

## 0.5.0 — COMPUTE / arithmetic expressions (PL08)

- Executes `COMPUTE target [ROUNDED] = <expr> [ON SIZE ERROR …]`.
- **Expression evaluation** over the parser's precedence-layered tree: `+ - * /`,
  `**` (exponentiation, right-associative, non-negative integer exponents;
  negative/fractional/oversized exponents are a clean error), unary sign, and
  parentheses. Names must resolve to numeric items.
- **`ROUNDED`** rounds half away from zero to the receiver's decimal places;
  without it the result truncates (toward zero), consistent with the other verbs.
- **`ON SIZE ERROR`** runs its statements and leaves the receiver unchanged when
  the result's integer part overflows the receiver, or when a division by zero
  occurs in the expression. Without a handler, overflow truncates high-order
  digits silently (as `MOVE` does) and a zero divisor stays a hard
  `DivideByZero` error.
- Division inside an expression is carried to a fixed 12-digit intermediate
  fractional precision, then rounded/truncated into the receiver — a documented
  simplification of the standard's composite intermediate-precision rules (see
  PL08); to be refined in a later PR.
- Exponentiation is bounded (`MAX_POW_EXP = 1024`) so a hostile `A ** huge`
  cannot spin the repeated-multiply loop.

## 0.4.0 — IF / conditional branching (PL08)

- `IF cond THEN… [ELSE …]` with the current grammar's simple relational
  condition (`[IS] [NOT] (GREATER [THAN] | LESS [THAN] | EQUAL [TO])`).
- Comparison is **numeric** when both operands are numeric (exact, digit-string
  based — any size, sign-aware, differing fraction lengths compare equal) and
  **alphanumeric** otherwise (space-padded to equal length, COBOL's rule);
  figurative constants take the other operand's category/length.
- Both branches may hold multiple statements; branches nest, and a `STOP RUN`
  inside a branch ends the whole program (statement execution now returns a
  stop-flag that unwinds nested IFs).
- Remaining control flow (`PERFORM`, `GO TO`, `EVALUATE`, `END-IF`) and `COMPUTE`
  stay deferred. Roadmap in PL08.
- **DoS hardening:** deeply-nested `IF … IF … IF …` (the first construct that
  nests) can no longer overflow the native stack — `cobol-parser` 0.1.1 opts into
  the parser's depth cap, so it returns a clean parse error end to end.
  Regression test added here too.


## 0.3.0 — DIVIDE (PL08)

- `DIVIDE a INTO b [GIVING g]` — result = b / a. Fixed-point division computed to
  the receiver's fractional precision and **truncated toward zero** (COBOL's
  behaviour absent `ROUNDED`): `10 / 3` into `9(3)V99` → `"00333"`.
- **Divide by zero** (no `ON SIZE ERROR` to catch it) surfaces as
  `RuntimeError::DivideByZero`, never a panic. Intermediate scaling uses checked
  `i128` arithmetic (overflow → error).
- Remaining arithmetic — `COMPUTE`, `ROUNDED`/`ON SIZE ERROR` (need frontend
  clauses) — and signed `S` numerics stay deferred. Roadmap in PL08.


## 0.2.0 — Fixed-point decimal arithmetic (PL08)

- `ADD` / `SUBTRACT` / `MULTIPLY` with the current grammar's forms
  (`ADD op… TO name [GIVING g]`, `SUBTRACT op… FROM name [GIVING g]`,
  `MULTIPLY a BY b [GIVING g]`).
- Exact fixed-point decimal maths on a scaled `i128`: addition/subtraction align
  by the implied decimal point (result keeps the wider fraction); multiplication
  sums the operands' fractional lengths. The result is then `MOVE`d into the
  receiver's picture, so COBOL's silent truncation applies. Overflow beyond ~38
  digits returns a `RuntimeError` (never panics or wraps).
- Unsigned receivers keep the magnitude (e.g. `SUBTRACT 5 FROM 3` stores 2) —
  signed `S` fields and `ROUNDED`/`ON SIZE ERROR` (which need frontend clauses)
  and `DIVIDE` remain deferred (descriptive errors). Roadmap in PL08.


## 0.1.0 — COBOL runtime, execution spine (PL08)

- `run_cobol(source) -> Result<String, RuntimeError>`: parse (via cobol-parser),
  lower the CST to a typed model, build a PICTURE-typed data model, execute, and
  return the captured `DISPLAY` output. I/O is captured (pure, testable).
- **Data model**: PICTURE parsing for unsigned numeric-display (`9`/`V`) and
  character (`X`/`A`) with `(n)` repetition; the item tree from level numbers
  (`01` groups, `02+` subordinates, `77` standalone); `VALUE` initialisation;
  figurative `ZERO`/`SPACE`.
- **MOVE** with exact COBOL receiving rules — numeric: decimal-aligned,
  integer right-justified/zero-filled/high-order-truncated, fraction
  left-justified/zero-filled/low-order-truncated; character: left-justified,
  space-padded/right-truncated.
- **DISPLAY** concatenates operand images with no separator; numeric items show
  raw stored digits (no implied decimal point). **STOP RUN**; paragraph
  fall-through.
- Honest scoping: signed numerics, editing pictures, `USAGE COMP`/`COMP-3`,
  group `MOVE`, name qualification, and every verb beyond `MOVE`/`DISPLAY`/`STOP
  RUN` return a descriptive `RuntimeError`. Roadmap in PL08.
