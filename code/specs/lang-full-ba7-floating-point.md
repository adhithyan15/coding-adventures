# LANG-FULL BA7 — Dartmouth BASIC floating-point (`f64`)

**Status:** design spec — **decision-complete** (all §7 questions resolved by
historical Dartmouth BASIC fidelity); implementation proceeds in slices BA7-1..3.
**Depends on:** **E3** (reals / `f64` — COMPLETE, every backend executes f64),
**E8** (numeric conversions `int_to_real` / `real_to_int_trunc` — COMPLETE), and
**BA2** (character-level `PRINT` via `putchar` — COMPLETE, the digit-printing
substrate this builds on).
**Unblocks:** real BASIC programs (averages, interest, `SQR`/`SIN`-style math
once those builtins land), and brings `dartmouth-basic-iir-compiler` in line with
the actual GE-225 Dartmouth BASIC semantics.

---

## 1. What this is

Dartmouth BASIC (1964) has **exactly one numeric type: floating-point.** The
grammar already says so:

> *"Dartmouth BASIC 1964 stores all numbers as floating-point internally. Even
> integers like 42 are stored as 42.0."* — `code/grammars/dartmouth_basic.tokens`

The lexer already recognises every real literal form (`42`, `3.14`, `.5`,
`1.5E3`, `1.5E-3`). But `dartmouth-basic-iir-compiler` currently **truncates
every number to `i64`** (`f as i64`) and runs the whole language on an integer
value model — a deliberate V1 limitation taken *"until the backends grow SSE2
support"* (module doc). E3 removed that limitation: every backend now executes
`f64`. BA7 is the cutover — make BASIC's value model **`f64`, end to end**, the
way the real language always was.

This is a **whole-value-model change**, so it gets a spec before code.

---

## 2. The value model: every BASIC number is `f64`

**Decision (from the language, not invented): a BASIC scalar variable, array
element, literal, and arithmetic result are all `f64`.** There is no integer
variable type in Dartmouth BASIC (no `%` suffix — that is a later Microsoft
dialect, explicitly out of scope).

Concretely, in `dartmouth-basic-iir-compiler`:

| Construct | Today (V1, i64) | BA7 (f64) |
| --- | --- | --- |
| number literal `3.14` | `const 3 : i64` (truncated!) | `const 3.14 : f64` |
| `LET A = …` | `mov A : i64` | `mov A : f64` |
| `A + B`, `*`, `-`, `/` | `add/... : i64` | `add/... : f64` (E3 ops) |
| `A = B`, `<`, `>`, `<=`… | `cmp_* : i64` operand width | `cmp_* : f64` operand width |
| `FOR I = 1 TO 10 STEP 0.5` | i64 counter | `f64` counter + step |
| `DIM A(n)` element | `array<i64>` | `array<f64>` |
| `DATA 3.14` | truncated to i64 | `f64` pool |
| `PRINT X` | integer digits (BA2) | **real formatting** (§4) |

Integer `i64` does **not** disappear from the emitted IIR — it survives in
exactly the places BASIC semantics require a whole number (§3).

---

## 3. Where integers still live (and how reals cross into them)

Three contexts are **syntactically or semantically integer**, even though the
value model is real. Each uses E8's `real_to_int_trunc` (or stays a literal int)
to cross the boundary — this is *why* BA7 depends on E8.

1. **Line numbers** (`GOTO 100`, `IF … THEN 100`, the `line_N` labels). These
   are program structure, not runtime values — they stay integer literals in the
   IIR (`jmp line_100`). No change.
2. **Array subscripts** (`A(I)`). BASIC arrays are indexed by a whole number; a
   real subscript is truncated (`A(2.7)` → element 2). Lower the subscript
   expression as `f64`, then `real_to_int_trunc` it to an `i64` index before
   `array_get`/`array_set`. (`DIM A(n)`'s bound `n` is a literal — parse it as an
   integer directly, as today.)
3. **The future `INT(x)` builtin** and any explicit truncation — `real_to_int_trunc`
   directly. (Not in BA7's first slice, but the op is ready.)

Everything else — every arithmetic value, comparison operand, `FOR` counter,
`PRINT` argument — is `f64`.

---

## 4. Real `PRINT` formatting (the key design decision)

This is the one genuinely new piece of logic. BA2 prints an integer by recursing
on `n / 10` and emitting digits through `putchar`. BA7 must print a **real**.
Dartmouth BASIC's rule (and the BA7 proposal):

- **Whole-valued reals print with no decimal point:** `PRINT 42.0` ⇒ `42`,
  `PRINT 6.0 * 7.0` ⇒ `42`. (So existing integer-valued programs — and the BA2
  matrix cells — keep their exact output.) Detected by `value == trunc(value)`
  and in range.
- **Non-whole reals print an integer part, `.`, and fractional digits:**
  `PRINT 3.14` ⇒ `3.14`, `PRINT 1.0 / 4.0` ⇒ `.25` (no leading zero — §7.1).
- **Sign:** a leading `-` for negatives (reuse BA2's sign handling).
- **6 significant digits, trailing zeros trimmed (§7.2):** the GE-225 printed 6
  significant decimal digits; emit digits until 6 significant ones are produced,
  trim trailing zeros (`0.5` ⇒ `.5`, not `.500000`), round-half-up the last kept
  digit.
- **Out-of-range magnitudes use `E` notation (§7.3) — implemented in BA7-2.**

### Lowering — extend, don't replace, the BA2 helper

BA7 adds a **`__basic_print_real(x: f64)`** synthetic helper alongside BA2's
`__basic_print_int`. It reuses the same `putchar` substrate and integer-digit
helper:

```text
fn __basic_print_real(x):
    if x < 0: putchar('-'); x = 0 - x
    ip   = real_to_int_trunc(x)            # integer part (E8)
    __basic_print_uint(ip)                 # BA2 helper prints the integer part
    frac = x - int_to_real(ip)             # fractional remainder (E8 + f64 sub)
    if frac > 0 (after rounding to N digits):
        putchar('.')
        emit up to N fractional digits:    # frac = frac*10; d = trunc(frac);
            putchar('0' + d); frac = frac - d   # repeat, stop at N or when 0
```

Built from ops that already run on all 7 backends — `f64` `sub`/`mul`/`cmp`,
`int_to_real`/`real_to_int_trunc` (E8), `putchar`, `call`, and the BA2 integer
helpers. **So BA7, like BA2, needs ZERO new backend ops.** (Rounding is done by
adding `0.5 / 10^N` before extraction, or by a final-digit round — see Q1.)

`emit_print` chooses `__basic_print_real` vs `__basic_print_int` by the static
type of the item (everything is `f64` in BA7, so `__basic_print_real` becomes the
default; `__basic_print_int`/`__basic_print_uint` remain as its building blocks).

---

## 5. Backward compatibility

The existing BASIC tests and the BA2/BA3/BA6 matrix cells use **whole-valued**
programs (`PRINT 42`, `1+2+3+4+5=15`, arrays of integers). Under the §4 rule,
whole-valued reals print **identically** (`42`, `15`, …), so:

- The matrix's existing BASIC `Stdout` cells stay green unchanged.
- The frontend's unit tests that assert i64 ops (`add`, `mul`) become `add`/`mul`
  at `f64` width — those assertions must be updated to the real value model
  (mirrors how E3 updated ALGOL's tests). The *observable output* is unchanged.

This is the BA2 lesson applied up front: changing the value model touches every
in-crate test harness and the `jit_smoke`/`jit_real_backend` capture helpers
(now decode the same `putchar` byte stream — no change needed there). Sweep them
all and run the **full** crate suite fresh, not just `--lib`.

---

## 6. Implementation plan (sliced PRs, each green on all 7 backends)

BA7 is bigger than BA2, so it ships in focused slices rather than one mega-PR:

- **BA7-1 — value model + arithmetic + whole-valued PRINT.** Stop truncating
  literals; carry `f64` through `LET`/arithmetic/`PRINT`; add `__basic_print_real`
  but only exercise whole-valued output first (so all existing cells stay green).
  Matrix cell: `PRINT 6.0 * 7.0` ⇒ `42` on all 7 backends — proving the f64 path
  runs end to end while output is unchanged. Update in-crate test assertions to
  the f64 model.
- **BA7-2 — fractional PRINT formatting + `E` notation.** Turn on the
  `.`-and-fraction path (6 significant digits, trailing-zero trim, round-half-up,
  no leading zero) and the out-of-range `E`-notation path (§7.3). Matrix cells:
  `PRINT 3.14` ⇒ `3.14`, `PRINT 1.0 / 4.0` ⇒ `.25`, `PRINT 0.0 - 2.5` ⇒ `-2.5`,
  plus an `E`-notation cell.
- **BA7-3 — reals in the rest of the language.** `f64` comparisons (`IF A < 1.5`),
  `FOR I = 1 TO 2 STEP 0.5`, `DIM`/array elements as `array<f64>` with
  `real_to_int_trunc` subscripts, and `f64` `DATA`/`READ`. One matrix cell each.
- **BA7-4 (optional) — `INT(x)` builtin** (`real_to_int_trunc`) and tidy-up.

Each slice: bump `dartmouth-basic-iir-compiler` version + CHANGELOG + README,
security-review (diff inline), push, babysit. BA7 marked ✅ when BA7-1..3 land.

---

## 7. Resolved decisions — honoring historical Dartmouth BASIC

These were open questions; they are **resolved here by fidelity to the original
GE-225 Dartmouth BASIC** (the project's rule: a historical language honors the
decisions the real language made, rather than a modernized variant). No further
sign-off needed — these are the spec.

1. **Leading zero — NO leading zero: `.25`, `-.25`.** The 1964 Dartmouth system
   printed a magnitude less than 1 without a leading `0` (`.25`, not `0.25`).
   Lowering: when the integer part is `0` and a fraction exists, skip the
   `__basic_print_uint(0)` call and emit `.` directly (still emit `0` for an
   exact `0`/`0.0`).
2. **Precision — 6 significant digits.** The GE-225 stored numbers in
   single-precision floating point (~6–7 significant decimal digits), and the
   interpreter printed **6 significant digits**, trailing zeros trimmed,
   round-half-up on the last kept digit. (This is *significant* digits across the
   whole number, not 6 fractional digits: `123.456`, `1.23457`, `.000123457`.)
   The fractional-emit loop counts significant digits, not places.
3. **Scientific notation — honored, sequenced to BA7-2.** The original DID switch
   to `E` notation for magnitudes outside roughly `1e-1 .. 1e6` (e.g.
   `1.23457E+08`, `1.23457E-04`). We **honor it** — it is not dropped — but
   implement it in **BA7-2** alongside the fractional formatter; **BA7-1** covers
   whole-valued and in-range decimals (the common case, and what keeps the
   existing matrix cells green). The roadmap notes E-notation as an explicit BA7-2
   deliverable, not an open question.
4. **`real_to_int_trunc` trap on out-of-range — kept.** A subscript or `INT()` of
   a real beyond `i64` range traps (E8 semantics). The original BASIC errored on
   an out-of-range subscript, so the trap is the faithful behavior.

*Carried over from BA2 (documented there, not reopened here):* the historical
leading-space-for-sign and trailing-space "print zone" column behavior remains a
separate print-zones follow-up on top of BA2's separator model; BA7 changes the
*number format*, not the *separator/zone* model.
