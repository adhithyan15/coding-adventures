# PL08 — COBOL Runtime (execution model)

## Overview

This spec defines how COBOL programs **execute**. The frontend ([PL07]) lexes
and parses COBOL; this layer runs it. It is the spine of the long-term goal — a
*full, faithful* COBOL implementation — because COBOL's defining quirks are
**runtime** behaviours, not syntax: fixed-point decimal arithmetic, PICTURE
editing on `MOVE`, `USAGE` storage (`DISPLAY`/`COMP`/`COMP-3`), level-88
condition-names, `PERFORM … THRU`, `GO TO … DEPENDING ON`, `ALTER`, and more. A
parser cannot implement a quirk; an interpreter must.

The runtime is a **tree-walking interpreter** over the `cobol-parser` CST
(mirroring the repo's other `*-runtime` crates, e.g. `s-runtime`). It lowers the
generic parse tree into a typed program model, builds a **PICTURE-typed data
model**, and executes the PROCEDURE DIVISION statement by statement.

**Rust crate:** `code/packages/rust/cobol-runtime/`.
**Public API:** `run_cobol(source: &str) -> Result<String, RuntimeError>` — runs a
program and returns everything it `DISPLAY`ed (the captured console). I/O is
captured, not written to a real console, so execution is pure and testable
(the same discipline the BASIC VM uses for its input/output).

## The data model (where the quirks begin)

COBOL has no generic "number" — every data item is a fixed-size field described
by a **PICTURE**. The stored representation *is* the quirk.

### PICTURE categories

| Symbol | Category | Meaning |
|--------|----------|---------|
| `9` | numeric | one decimal digit position |
| `V` | numeric | implied decimal point (occupies **no** storage) |
| `S` | numeric | operational sign (default: overpunch on the trailing digit — no extra storage) |
| `X` | alphanumeric | any character |
| `A` | alphabetic | a letter or space |
| `(n)` | — | repeat the preceding symbol `n` times (`9(3)` = `999`) |

A picture's **size** is its count of storage-bearing character positions: for a
numeric picture that is (integer digits + fractional digits) — `V` and a
default `S` add none; for `X`/`A` it is the character count.

### Storage representation

- **Numeric-display** (`9`, `V`): stored as exactly (int + dec) **digit
  characters**, with the decimal point *implied* (never stored). `PIC 9(3)V99`
  holding 42.5 stores `"04250"` — and that is exactly what `DISPLAY` shows.
- **Alphanumeric / alphabetic** (`X`, `A`): stored as `size` characters.

### `MOVE` — the signature quirk

`MOVE source TO receiver` reshapes the source to fit the receiver's picture. The
rules depend on the receiver's category:

- **Alphanumeric/alphabetic receiver:** the source's characters are placed
  **left-justified**; a shorter source is **space-padded on the right**, a
  longer source is **truncated on the right**. (`JUSTIFIED RIGHT` reverses this —
  future work.)
- **Numeric receiver:** the source is **aligned by the decimal point**. The
  integer part is **right-justified** into the integer positions (zero-filled on
  the left, high-order digits **truncated** if they overflow); the fractional
  part is **left-justified** into the decimal positions (zero-filled on the
  right, low-order digits **truncated**). Silent truncation is a real, famous
  COBOL foot-gun and is implemented faithfully.

Figurative constants move as themselves: `ZERO` fills with `0`s, `SPACE`/`SPACES`
fills with spaces.

### `DISPLAY`

`DISPLAY op1 op2 …` writes the **concatenation** of its operands' display images
(items → their stored characters; literals → their text) followed by a newline.
Crucially there is **no separator** between operands — a common surprise. A
numeric item displays its raw stored digits, with **no** decimal point.

## Execution model

- The PROCEDURE DIVISION is paragraphs → sentences → statements. Execution runs
  statements top to bottom, **falling through** paragraph boundaries, until
  `STOP RUN` (or the end of the program).
- WORKING-STORAGE items are initialised before execution: an item with a `VALUE`
  clause is initialised by `MOVE`-ing that literal into it; an item without a
  `VALUE` is initialised to zeros (numeric) or spaces (character). (The 1960
  standard leaves un-`VALUE`d storage undefined; a deterministic interpreter must
  pick, and zeros/spaces is the conventional, least-surprising choice.)

## Scope — v0.1 (this PR)

A **small but fully correct** slice, establishing the data model and execution
spine. No stubs: anything not yet modelled returns a descriptive `RuntimeError`
rather than producing wrong output.

**Implemented, faithfully:**
- PICTURE parsing for pure **`9`(`V`)** unsigned numeric-display and pure
  **`X`** / **`A`** character pictures, with `(n)` repetition.
- The item tree from WORKING-STORAGE level numbers (`01` groups, `02+`
  subordinates, `77` standalone); group items display as the concatenation of
  their elementary children.
- `VALUE` initialisation; figurative `ZERO`/`SPACE`.
- `MOVE` (literal / item / figurative → elementary item) with the exact
  alphanumeric and numeric receiving rules above.
- `DISPLAY` (items and literals, concatenated, newline-terminated).
- `STOP RUN`.

**Added in v0.2 / v0.3:** fixed-point decimal `ADD` / `SUBTRACT` / `MULTIPLY` /
`DIVIDE` (the current grammar's `TO`/`FROM`/`BY`/`INTO` + `GIVING` forms) —
decimal-point aligned, truncating toward zero into the receiver, unsigned
receivers keeping the magnitude; overflow beyond ~38 digits is an error (never a
panic), and divide-by-zero surfaces as `RuntimeError::DivideByZero`.

**Deferred to later PRs (return a descriptive `RuntimeError` for now):**
signed `S` numerics and overpunch-sign display; `P` scaling; `COMPUTE` and
`ROUNDED`/`ON SIZE ERROR` (which need frontend clauses); editing pictures
(`Z * $ , . + - CR DB B 0 /`); `USAGE COMP`/`COMP-3` (binary / packed decimal);
group `MOVE`; name qualification (`OF`/`IN`); `REDEFINES`/`OCCURS`; and every
verb not listed above.

## Roadmap toward full COBOL

Each item is a run-verified PR; the runtime grows one quirk at a time.

1. **v0.1 — execution spine** (merged): data model + `MOVE`/`DISPLAY`/`STOP RUN`.
2. **v0.2 — arithmetic** (merged): fixed-point `ADD`/`SUBTRACT`/`MULTIPLY`.
3. **v0.3 — `DIVIDE`** (merged): fixed-point division + divide-by-zero.
4. **v0.4 — `IF … ELSE`** (merged): conditional branching; numeric and
   alphanumeric comparison; `STOP RUN` unwinds nested branches.
5. **v0.5 — `COMPUTE`** (this PR): evaluate precedence-layered arithmetic
   expressions (`+ - * / **`, unary sign, parentheses) over the parser's tree;
   `ROUNDED` (half away from zero) vs default truncation; `ON SIZE ERROR` on
   integer overflow or division by zero (receiver unchanged, handler runs;
   without a handler, overflow truncates silently and a zero divisor is a hard
   error). **Known simplification:** intermediate **division** precision is a
   fixed 12 fractional digits, not the standard's composite intermediate-precision
   rule; `**` supports non-negative integer exponents only (bounded at 1024).
   Both are flagged for a later precision/exponent-faithfulness PR.
6. **v0.6 — signed numerics** (this PR): `PIC S9…` fields carry an operational
   sign through `MOVE`/arithmetic/`COMPUTE`; `DISPLAY` renders it as the default
   trailing "zoned" overpunch on the units digit (`−123` → `12L`). The explicit
   `SIGN` clause and its `SEPARATE`/`LEADING` variants are deferred.
7. **v0.7 — `PERFORM para [n TIMES]`** (merged): out-of-line paragraph
   invocation with return, an integer repeat count (`≤ 0` runs zero times), and a
   recursion-depth guard against a self-performing paragraph.
8. **v0.8 — `GO TO para`** (merged): unconditional transfer, with the procedure
   division executed as a **program counter** over paragraphs (so back-edge
   `GO TO` loops run iteratively, no stack growth). A `GO TO` out of a performed
   paragraph transfers at the top level. `GO TO … DEPENDING ON` and `ALTER` are
   deferred.
9. **v0.9 — `PERFORM … UNTIL`** (merged): a conditional loop, testing the
   condition **before** each iteration (`WITH TEST BEFORE`), driven iteratively.
10. **v0.10 — `PERFORM … VARYING`** (merged): a counted loop over an induction
    variable (`VARYING id FROM start BY step UNTIL cond`) — `id := start`, run
    while `cond` is false, step `id` by `step` after each iteration. The repeat
    forms are modelled as a `PerformMode` enum.
11. **v0.11 — `PERFORM … THRU`** (merged): run a range of paragraphs
    (`para-1 THRU para-2`, source order, fall-through between) as the perform
    body; composes with every repeat mode. Backwards ranges are a clean error.
    `WITH TEST AFTER` and the inline form remain deferred.
12. **v0.12 — `ROUNDED`/`ON SIZE ERROR` on the arithmetic verbs** (this PR):
    `ADD`/`SUBTRACT`/`MULTIPLY`/`DIVIDE` now accept both clauses, matching
    `COMPUTE`, via a shared `store_result` path (round → size-error → store). A
    zero `DIVIDE` divisor is a size-error condition when a handler is present.
12. **Editing pictures** on `MOVE`/`DISPLAY` (`Z`/`*`/`$`/`,`/`.`/`+`/`-`/`CR`/`DB`).
13. **Rest of control flow** — `END-IF`, `EVALUATE`, inline `PERFORM`,
    `PERFORM … WITH TEST AFTER`, `GO TO … DEPENDING ON`, `ALTER`.
8. **Conditions** — level-88 condition-names.
9. **Tables** — `OCCURS`, subscripts, `REDEFINES`, `USAGE`.
8. **File I/O** — `SELECT`/`FD`, sequential then indexed/relative, `OPEN`/`READ`/
   `WRITE`/`REWRITE`/`CLOSE`.
9. **Later standards** — layer COBOL-61/68/74/85/2002/2014 features on the solid
   COBOL-60 base (`END-IF` scope terminators, `EVALUATE`, inline `PERFORM`,
   reference modification, intrinsic functions, free-form format, OO, …).

## Test Strategy

Every runtime feature is proven by **running a program and asserting its exact
`DISPLAY` output** — the only real proof a quirk is implemented. v0.1 tests:

- A character `MOVE` that space-pads (`MOVE "HI" TO PIC X(5)` → `"HI   "`) and one
  that truncates on the right.
- A numeric `MOVE` that zero-fills and right-justifies (`MOVE 42 TO PIC 9(5)` →
  `"00042"`), and one that truncates high-order and low-order digits with an
  implied decimal (`MOVE 123.456 TO PIC 9(2)V9` → `"230"`).
- `DISPLAY` concatenates operands with no separator; a numeric shows raw digits.
- `VALUE` initialisation; figurative `ZERO`/`SPACES`.
- A complete program (IDENTIFICATION + WORKING-STORAGE + PROCEDURE) runs end to
  end and produces the expected multi-line console.
- Unsupported constructs (e.g. `ADD`, signed `S9`) return a clear error, not
  wrong output.

[PL07]: PL07-cobol-60.md
