# Changelog

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
