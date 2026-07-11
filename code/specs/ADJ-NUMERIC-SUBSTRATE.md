# ADJ-NUMERIC-SUBSTRATE — arbitrary precision is the default; f64 is a labeled, lossy export

**Status:** Spec-first. Foundational numeric core underneath the ADJ reasoning engine
(compute/rules/constraints/formulas/tables). Changes the default arithmetic from
`f64` + a bounded `i128` sidecar to **arbitrary-precision Big numbers**.
**Author:** numeric-precision pass, 2026-07-11.

**Decision (owner):** the default ways ADJ computes are **BigInteger, BigRational,
BigDecimal, BigDouble**; the underlying math is done at **arbitrary precision**, and a
value is converted to a *specific* precision/format (fixed decimals, significant
figures, scientific notation) only **explicitly, at the boundary**.

**Why it matters:** a single percentage point can be worth hundreds of millions of
dollars. A silently-rounded `f64` that flips that point is precisely the *accounting
failure* the byte-provenance thesis exists to prevent. Precision must be **exact by
default and lossy only on purpose, auditably**.

---

## 1. What's wrong today (grounded)

The compute engine (`logic-engine/src/compute.rs`) is **`f64`-primary** with a
best-effort **exact-rational sidecar** `ExactRational { num: i128, den: i128 }`. The
sidecar is **bounded** (goes `None` on `i128` overflow, on roots, on transcendentals,
and beyond `MAX_EXACT_POW = 1024`), so the *primary* magnitude a consumer sees is `f64`
— which silently loses precision. That is unacceptable for decision-critical arithmetic.

A `numeric-tower` crate exists (BigInt/BigRational/Decimal) **but** (a) it is backed by
third-party `num-bigint`/`num-rational` — repo policy treats third-party deps as debt to
retire (cf. the flagged `engram-anki` rusqlite/prost/zstd violation); (b) it has **no
arbitrary-precision float** (`BigDouble`); and (c) the **ADJ compute engine does not use
it**. So the reasoning core needs its own **zero-dependency** Big substrate.

---

## 2. The types — a zero-dep `bignum-core` suite

Built from scratch (as the CAS, latex, and sqlite crates were — repo canon is zero
third-party deps). Each is its own crate/module, exhaustively tested (differential vs a
known-good oracle where feasible).

- **`BigInteger`** — arbitrary-precision signed integer (sign + limb vector). Full
  add/sub/mul/div/rem, `gcd`, `pow`, comparison, parse/format (incl. any base). The
  foundation for the rest.
- **`BigRational`** — `BigInteger` numerator/denominator, always gcd-normalized. **Exact**
  `+ − × ÷` with no rounding, unbounded. Replaces the bounded `i128 ExactRational` — the
  default for all four arithmetic primitives.
- **`BigDecimal`** — a `BigInteger` mantissa + a base-10 scale. Exact base-10 arithmetic
  for **money, percentages, tax, dosing** — anything where base-10 exactness is the point
  and a binary float would misrepresent `0.1`.
- **`BigDouble`** — a `BigInteger` mantissa + a base-2 exponent + a **carried working
  precision and rounding mode** — arbitrary-precision binary floating point for the values
  that are *genuinely irrational* (`sqrt`, `ln`, `exp`, trig): computed **to a requested
  precision** with a stated rounding mode and guard digits, not silently truncated to 53
  bits. A `BigDouble` **knows and carries** how many correct digits it has.

Conversions are explicit and **directional by safety**: exact → exact is free;
exact → `BigDouble`(p) states a precision; anything → `f64` is a **clearly-labeled lossy
narrowing**, never implicit in a computation.

---

## 3. Exactness discipline — lossy only on purpose, and audited

Every computed value knows whether it is **EXACT** (`BigInteger`/`BigRational`/`BigDecimal`)
or **APPROXIMATE to precision _p_** (`BigDouble`), and the audit records it.

- `+ − × ÷` of exacts **stay exact** (`BigRational`), unbounded — `1/3` is `1/3`, not
  `0.333…`; `(0.1 + 0.2)` is exactly `3/10`.
- A **root or transcendental** yields a `BigDouble` at a **requested precision** (default
  high, e.g. 50 significant digits, configurable per call); the audit says *"accurate to N
  significant figures, rounding mode R"*.
- **No silent lossy coercion.** Mixing an exact and an approximate widens to
  `BigDouble`(p) and the result is *labeled* approximate. `f64` never appears mid-computation;
  it is only a boundary export a consumer explicitly asks for, tagged lossy.
- The **provenance/audit chain carries exactness + precision** alongside the value, so
  "is this number exact?" and "to how many figures?" are answerable from the trail — and
  `adj-verify` re-checks that the claimed precision holds.

---

## 4. Precision & formatting at the boundary (stdlib + ops)

Converting to a *specific* precision/format is an **explicit, auditable** final step —
never a silent middle step. Provide, as engine ops and grounded stdlib formulas:

- `round_to(x, places)`, `round_sig(x, figures)` — round an exact/`BigDouble` to N decimal
  places or N significant figures, stating the rounding mode; the result records "rounded
  from <exact> to N figs".
- `to_scientific(x [, figures])` — scientific / engineering notation (`6.022e23`), with a
  stated mantissa precision.
- `percent(part, whole)` / `to_percent(x [, places])` — a percentage to a stated precision
  (the `$100M-per-point` case: computed exactly, formatted deliberately).
- `to_currency(x, code [, places])` — base-10-exact money formatting (`BigDecimal`).
- Each is provenanced (its definition) and each records the **exact source value** it was
  narrowed from, so the audit shows both the exact number and its rendered form.

---

## 5. Engine integration

The ADJ compute `Value` becomes a **Big numeric** — `BigRational` by default; `BigDecimal`
for declared base-10 quantities (money/percent); `BigDouble`(p) for irrationals. The
bounded `ExactRational` sidecar is **subsumed**. `f64` magnitudes remain available as a
**labeled lossy export** for legacy consumers only. Golden pins keep rendering
(`bmi(70,1.75)` still shows `22.857…`), but the underlying value is now the exact
`3200/140` = `160/7` rational, and `22.857142857142858` is a *formatted `f64` export*, not
the ground truth — the audit exposes the exact form.

The rule substrate (ADJ-RULE-SUBSTRATE) computes on this: a formula/rule body evaluates in
Big arithmetic; a constraint solves over exact rationals where the backend allows;
`adj-verify` re-computes in Big and confirms the rendered precision.

---

## 6. Rung staging (each: spec-sync → tests → impl → security-review → babysit; ZERO deps)

- **NUM-1 — `BigInteger`** (zero-dep): the integer core + differential tests.
- **NUM-2 — `BigRational`**: exact `+−×÷`, gcd-normalized; a drop-in richer than
  `ExactRational`.
- **NUM-3 — `BigDecimal`**: base-10 exact (money/percent).
- **NUM-4 — `BigDouble`**: arbitrary-precision float + `sqrt`/`ln`/`exp`/trig **to a
  requested precision** with a stated rounding mode; carries its correct-digit count.
- **NUM-5 — engine adopts Big-by-default**: ADJ compute `Value` = Big numeric; `f64`
  becomes a labeled boundary export; `ExactRational` subsumed; golden pins hold (as
  formatted exports); exactness recorded on `Derived`.
- **NUM-6 — precision/format ops + stdlib formulas** (`round_to`, `round_sig`,
  `to_scientific`, `to_percent`, `to_currency`) + audit exactness + `adj-verify` precision
  re-check.
- **Later — retire `numeric-tower`'s `num-bigint`** onto `bignum-core` (pays down the
  existing third-party debt; out of this spec's critical path).

**Sequencing:** foundational and mostly **parallelizable** with the rule-substrate rungs
(RS-*); the engine swap (NUM-5) is the integration point and should land **before the
clinical apex (FL-5)** so money, percentages, and dosing are exact by construction. NUM-1
has no dependency and can start immediately.

---

## 7. Verification & invariants

- **Zero third-party deps** in `bignum-core` and in the ADJ core after NUM-5 (`cargo tree`
  shows none new).
- **Exactness never silently lost:** a test suite proves `+−×÷` stay exact and that any
  narrowing is explicit and labeled; a "percentage-point-critical" worked example computes
  exactly and only rounds at `to_percent`.
- **Golden pins hold** as formatted exports; the exact form is recoverable from the audit.
- **`BigDouble` honesty:** an irrational's claimed precision is verified (guard-digit /
  interval check); `adj-verify` re-derives and confirms the rendered figures.
- `cargo test`/`cargo clippy` green per crate; the shipped formula/rule libraries keep
  their surface and their (now exact) values.
