# Changelog

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
