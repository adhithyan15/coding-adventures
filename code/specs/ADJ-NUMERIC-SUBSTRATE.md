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
  **256 bits ≈ 77 significant decimal digits** — far beyond `f64`'s ~15.9 and any real
  measurement; a fixed default now, made per-`KnowledgeBase` configurable in NUM-6); the
  audit says *"accurate to N significant figures, rounding mode R (round-half-even)"*.
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

### 4.1 Surface — a recognised built-in over the existing application grammar

`round_to`/`round_sig` are written as **native applications**, reusing the exact comma-list
call grammar that user formula applications already use (`quotient(a, b)`):

```
round_to(x, n)      % round x to n decimal PLACES        (NUM-6a)
round_sig(x, n)     % round x to n significant FIGURES    (NUM-6b)
```

where `n` is a **non-negative integer literal** no larger than the precision cap (100 places —
a DoS bound far beyond §3's 256-bit default precision). They are **recognised by name** during
`Apply` lowering, *before* the user-formula lookup, so they need no formula definition, no new
grammar production, and no LaTeX change — the application parser already accepts them. A
non-integer, negative, oversized, or non-literal `n` is a **compile error**, never a silent
truncation. The existing integer roundings are unchanged: `round_to(x, 0)` is exactly the
nearest-integer `Round` (also reachable as `⌊x⌉` / `\operatorname{round}(x)` through the
`latex "…"` frontend).

> **Surface note (divergence from the first §4.1 draft).** The kickoff spec proposed the
> two-argument *LaTeX* operator-name form `\operatorname{round}(x, n)`. Implementation (NUM-6a)
> found that the `latex` frontend does **not** keep an operator-name adjacent to a
> **comma-separated** argument list — a top-level comma splits the expression into a sequence,
> dropping the `round` — because `round` is not registered as an argument-taking function in the
> `latex` crate (unlike `\min`/`\max`, which are). Registering it there is a separate cross-crate
> change. The **native application grammar** already parses `name(a, b)` comma-lists correctly, so
> `round_to(x, n)` as a recognised `Apply` built-in is the surface that works today with zero new
> grammar — the same "no second way to write a call" spirit, on the application surface rather
> than the LaTeX one.

### 4.2 Rounding mode — stated, defaulting to round-half-even

Each narrowing states its **rounding mode**, defaulting to **round-half-even** (banker's
rounding) to match §3's exactness discipline and `bignum-core::RoundingMode::HalfEven`. This
is a *deliberate* difference from the legacy integer `Round`/`⌊x⌉`, which rounds ties **away
from zero** — which is precisely why `round_to` records its mode in the audit rather than
leaving it implicit: two roundings of the same exact value under different modes are both
honest and distinguishable from the trail.

### 4.3 Audit record — rounding is a first-class narrowing

Every application records, alongside its own provenance (the op's definition): (a) the
**exact source value** (the `Rational`/`Decimal` it narrowed, per §3), (b) the **target
precision** (`n` places or `n` significant figures), (c) the **rounding mode**, and (d) the
**rendered result**. `adj-verify` re-rounds the exact value under the recorded mode/precision
and confirms the rendered result — so a rounded number is auditable back to the exact one it
came from, never asserted. Rounding is thus an explicit, checkable step in the trail, not a
silent lossy coercion (§3).

> **Shipped (NUM-6v).** The re-check is implemented end-to-end. Each narrowing
> `DerivationNode` carries its operand's exact source (`operand_exact`), and
> `logic_engine::recheck_narrowing` / `recheck_narrowings` re-run the *same* exact narrowing
> primitive on that source under the recorded mode, confirming the recorded numeric result and
> (for the formatters) the rendered string reproduce. `adj-verify` walks every `let`-bound
> derived value's tree and re-checks every narrowing; a disagreement is a hard failure
> (`verified: false`, non-zero exit, named in `first_failure`). An operand with no exact
> sidecar (a transcendental result) is honestly reported `unverifiable`, never a pass.

### 4.4 Engine shape + sub-staging

`ComputeExpr::Unary(ComputeOp, …)` carries no parameter, so the precision-carrying roundings
add a new node `ComputeExpr::Round { spec: RoundSpec, mode: RoundingMode, expr }` where
`RoundSpec = Places(u32) | SigFigures(u32)`. It is **dimension-preserving** (like the unary
round family) and evaluated on the **exact path**: terminating cases via
`BigDecimal::round_to_scale`, repeating rationals (e.g. `1/3 → 0.33`) via `BigDecimal::div_round`
to the target scale — both already in `bignum-core` — then back to an exact `Rational` carrying
the recorded exact-source sidecar for the audit. NUM-6 lands in focused PRs, each spec-sync →
tests → impl → security-review → babysit:

- **NUM-6a** — `round_to` (decimal places) ✅ **shipped** (#8806): the
  `Round`/`RoundSpec::Places` engine node, the native `round_to(x, n)` application surface (see
  §4.1's surface note), exact eval, the §4.3 audit record, and an end-to-end formula test.
- **NUM-6b** — `round_sig` (`RoundSpec::SigFigures`) ✅ **shipped**: derives the target place
  count `n − 1 − ⌊log₁₀|x|⌋` from the value's most-significant-digit exponent (computed exactly
  from big-integer digit counts) and reuses 6a's exact eval. Native `round_sig(x, n)` surface,
  `n ≥ 1`.
- **NUM-6c** — the formatters `to_scientific` / `to_percent` / `to_currency` (rendering, on the
  6a/6b core) and per-`KnowledgeBase` `BigDouble` precision (the configurable default §3 defers
  here). Lands incrementally: **`to_scientific(x [, figures])`** ✅ **shipped** — the
  `ComputeExpr::ToScientific { figures, mode, expr }` engine node (a *rendering* op: it narrows
  to `figures` significant figures on the 6b exact path, then produces the normalized `d.ddde±E`
  string beside the narrowed exact value, both from one rounding so they can never disagree), the
  native `to_scientific(x [, figures])` application surface (optional `figures`, default 6; `≥ 1`,
  within the precision cap), the §4.3 audit record (`node:to_scientific`, `figures`, `mode`,
  `rendered`, operand subtree), and an end-to-end formula test. **`to_percent(x [, places])`**
  ✅ **shipped** — `ComputeExpr::ToPercent { places, mode, expr }`: takes `x` as a dimensionless
  ratio, scales by 100 and rounds to `places` decimal places on the exact path, renders the
  fixed-point `d.dd%` string, and carries the narrowed *fraction* as the numeric value
  (`"33.33%"` → `3333/10000`); native `to_percent(x [, places])` surface (optional `places`,
  default 2; `≥ 0`, so `to_percent(x, 0) = "50%"`), the §4.3 audit record (`node:to_percent`),
  and an end-to-end formula test. **`to_currency(x, code [, places])`** ✅ **shipped**, closing
  the formatter trio — `ComputeExpr::ToCurrency { code, places, mode, expr }`: rounds `x` to
  `places` decimal places on the exact base-10 path (`C = round(x·10^places)` via `BigDecimal`,
  no `f64` hop), renders the fixed-point `CODE d.dd` string (leading-zero padded; `places = 0`
  drops the point, e.g. JPY), and carries the narrowed *fraction* as the numeric value
  (`to_currency(100/3, USD, 2)` → `"USD 33.33"`, exact `3333/100`); native
  `to_currency(x, code [, places])` surface — the `code` is a bare identifier (lexed lowercase,
  normalized to the canonical uppercase ISO-4217 form; a non-identifier code is a compile error),
  `places` optional (default 2, `≥ 0`) — the §4.3 audit record
  (`node:to_currency`, `code`, `places`, `mode`, `rendered`, operand subtree), and an
  end-to-end formula test.
- **NUM-6v** — the **`adj-verify` precision re-check** ✅ **shipped** (§4.3, §7): each narrowing
  node carries its operand's exact source (`operand_exact`), the engine exposes
  `recheck_narrowing`/`recheck_narrowings`, and `adj-verify` re-rounds every narrowing in every
  derived value's tree, failing hard on any disagreement. This is the audit-exactness leg of
  NUM-6 — a rounded/formatted number is now re-derivable from the exact source it came from, not
  asserted.
- **NUM-6 is otherwise complete.** The one item its own §4.4 text used to defer — per-`KnowledgeBase`
  `BigDouble` precision — is **not** a NUM-6 sub-letter: it needs the `Number::Real(BigDouble)`
  compute path of §5, which is not wired into the engine, so it is its own rung. See **§8, NUM-7**.

> **Naming note (NUM-5, disambiguated).** "NUM-5" is used two ways across specs and they are
> **not** the same deliverable. §6 below describes NUM-5 as "engine adopts Big-by-default" — the
> **full** `Value = Number` tower swap (§5), where `f64` stops being a stored field and
> `ExactRational` is subsumed. That has **not** shipped; there is no `✅` marker for it anywhere in
> this document. `ADJ-EXACT-NUMBERS.md` (lines 11–14, 27–29) separately calls its own prerequisite
> "NUM-5" too, but describes something narrower that **has** shipped: making the *compute sidecar*
> itself exact and unbounded (`ExactRational(BigRational)`, `compute.rs:67` — upgraded off the old
> bounded `i128` pair). Read "NUM-5" in each document as scoped to that document; NUM-7 (§8) does
> not reuse the number, to avoid adding a third meaning.

---

## 5. Engine integration

The ADJ compute `Value` becomes a **Big numeric tower** — a single `Number` enum with three
variants and a promotion lattice `Decimal ⊂ Rational ⊂ Real`:

```
enum Number {
    Rational(BigRational),   // the DEFAULT: exact fractions, integer & ratio arithmetic (1/3, 160/7)
    Decimal(BigDecimal),     // declared base-10 quantities — money, percent, dosing (carries scale)
    Real(BigDouble),         // genuinely-irrational results — √, ln, exp, trig — at 256-bit default
}
```

**Promotion rules for a binary op** (so mixing is total and never silently lossy):

- `Decimal ⊕ Decimal` for `+ − ×` → `Decimal` (base-10 identity/scale preserved, still exact).
- Anything else over `{Rational, Decimal}` (including `Decimal ÷ Decimal`, which need not
  terminate in base 10) → `Rational` — the common exact supertype, since every `BigDecimal`
  `m·10⁻ˢ` is exactly the rational `m / 10ˢ`.
- Any operand `Real`, **or** an irrational op (`√`, transcendental) on any operand →
  `Real` at the max carried precision (default 256 bits). **`Real` is contagious**: once a
  value is inexact it stays inexact, and the audit labels it approximate-to-precision-_p_.

The bounded `i128 ExactRational` sidecar is **subsumed** by `Number` (unbounded, never drops
to `None`). `f64` is no longer a stored field — it is a **labeled lossy export**
(`Number::to_f64()`), only produced when a boundary consumer explicitly asks. Golden pins
keep rendering (`bmi(70,1.75)` still shows `22.857…`), but the underlying value is now the
exact `3200/140` = `160/7` rational, and `22.857142857142858` is a *formatted lossy export*,
not the ground truth — the audit exposes the exact form. Each `Derived`/`DerivationNode`
records its `Number` (hence its **exactness class**: EXACT for Rational/Decimal, APPROX(p)
for Real).

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
  re-check. Surface, mode, audit record and engine shape pinned in §4.1–§4.4; lands in
  focused sub-PRs **NUM-6a** (`round_to`) → **NUM-6b** (`round_sig`) → **NUM-6c** (formatters).
  ✅ **shipped in full** (NUM-6a/6b/6c/6v all landed; see §4.4).
- **NUM-7 — a constrained `Real`/`BigDouble` audit companion, per-`KnowledgeBase` precision.**
  NOT the full NUM-5 tower swap (see the naming note in §4.4) — a small, additive first rung
  that wires `bignum-core::BigDouble` into `compute.rs` for `sqrt` only, alongside the existing
  `f64` value and `ExactRational` sidecar (nothing existing changes shape). See §8 for the full
  design; sub-staged **NUM-7a** (bignum-core primitive + KB setting) → **NUM-7b** (engine wiring
  + audit JSON) → **NUM-7c** (`adj-verify` recheck).
- **Later — retire `numeric-tower`'s `num-bigint`** onto `bignum-core` (pays down the
  existing third-party debt; out of this spec's critical path).
- **Later still — the full NUM-5 tower swap** (§5) and transcendentals (`ln`/`exp`/`sin`/`cos`/…)
  in `bignum-core`/NUM-7, once a genuine need for either surfaces. Neither is scheduled.

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

---

## 8. NUM-7 — a constrained `Real`/`BigDouble` audit companion

**Scope decision.** §5's full `Number` tower is a breaking swap of the compute `Value` type —
every `DerivationNode` variant, `adj-verify`'s recheck logic, and every consumer crate would need
to move off `f64`+`ExactRational` at once. That is its own, larger, unscheduled effort (§6,
"Later still"). NUM-7 instead wires just enough of `BigDouble` into the engine to deliver §3's
per-`KnowledgeBase` precision promise, **additively**: nothing about the existing `f64` value or
`ExactRational` sidecar changes shape or meaning.

- **Scoped to `sqrt` only.** `bignum-core::BigDouble` (NUM-4) is correctly-rounded for
  `+ − × ÷ √`, but has **no transcendentals** (`ln`/`exp`/`sin`/`cos`/… are explicitly out of
  `bignum-core`'s scope today). So `sqrt` (reached via `x ^ 0.5` — the native lowering of both
  `\sqrt{x}` and `\sqrt[n]{x}`, adj-lang's LaTeX frontend) is the only op that gets a `Real`
  companion this rung. Wiring transcendentals is a **future rung**, blocked on `bignum-core`
  gaining correctly-rounded implementations of them first — that prerequisite is new information
  from this rung, not previously flagged.
- **Additive, not contagious.** A `sqrt` result's audit gains an optional `Real` companion (the
  `BigDouble` value, its precision, and its rounding mode) alongside the unchanged `f64` `result`.
  Further arithmetic on that value does **not** propagate the companion (`sqrt(4) + 1` has no
  `Real` companion on the `+`) — that promotion-lattice contagion is §5's job, not NUM-7's. A
  perfect square (`sqrt(4)`) still gets a companion like any other `sqrt` — NUM-7 does not special
  -case "is this base a perfect square," to avoid a second detection path.
- **Precision — the first per-`KnowledgeBase` setting.** `KnowledgeBase` gains a
  `real_precision_bits` field (default 256, per §3) plus builder/setter methods — this is the
  struct's first configuration field (previously: only facts/rules/derived data). No new `.adj`
  grammar exposes it per-program yet; setting it is a Rust-API-only KB construction step this
  rung. A future rung could add a program-level directive (e.g. `precision 512` at the top of a
  `.adj` file).
- **Audited, not just computed.** The `Real` companion is captured with the exact rational it was
  promoted from and its rounding mode, so `adj-verify` can independently re-promote-and-`sqrt` and
  confirm the recorded value — reusing the existing narrowing-recheck machinery (§4.3), even
  though a promotion to `Real` is technically a *widening* (exact → approximate) rather than a
  narrowing.

**Sub-staging** (each: spec-sync → tests → impl → security-review → babysit):

- **NUM-7a** — `bignum-core::BigDouble::from_rational` (the exact-rational → `BigDouble`
  promotion primitive, generalizing the existing private-use pattern inside
  `BigRational::to_f64()` to a caller-supplied precision) + `KnowledgeBase::real_precision_bits`
  (default 256, clamped to `bignum_core::MAX_PRECISION` since `BigDouble`'s internal precision
  guard panics outside range).
- **NUM-7b** — engine wiring: a new `real: Option<RealCompanion>` field on
  `DerivationNode::Op` (additive; `Round`/`ToScientific`/`ToPercent`/`ToCurrency` and their
  `ExactRational`-typed audit fields are untouched), populated when a `Pow` node's evaluated
  exponent is bit-exactly `0.5` and its base carried an exact sidecar. CLI JSON gains an additive
  `"real"` key when present.
- **NUM-7c** — `adj-verify` recheck: an `Op` node with a `Real` companion is re-promoted and
  re-`sqrt`'d at its recorded precision/mode and compared, reported through the existing
  `NarrowingCheck` machinery.
