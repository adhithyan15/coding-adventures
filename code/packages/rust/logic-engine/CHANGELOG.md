# Changelog

## [0.45.0] — 2026-07-22 — NUM-6a: the `round_to` precision narrowing (exact + audited)

Implements the compute-engine half of NUM-6a (`ADJ-NUMERIC-SUBSTRATE.md` §4.1–§4.4):
`round_to(x, n)` — round a value to `n` decimal places as an **explicit, checkable**
step, never a silent lossy coercion.

### Added

- `ComputeExpr::Round { spec: RoundSpec, mode: RoundingMode, expr }` — a precision
  narrowing distinct from the unary rounding family (`Abs`/`Floor`/`Ceil`/`Round`),
  because it carries a precision and a mode a bare unary op cannot hold. `RoundSpec`
  ships the `Places(u32)` variant (NUM-6b adds `SigFigures`).
- `DerivationNode::Round { spec, mode, operand, result }` — the audit record: the
  precision, the stated mode, and the operand subtree it narrowed, so `adj-verify`
  can re-round the operand's **exact** value and confirm the rendering.
- Re-export of `bignum_core::RoundingMode` so consumers can name a rounding mode
  without depending on `bignum-core` directly.

### Behaviour

- Rounding runs on the **exact rational** path: `n / d` is divided to `n` places via
  `bignum-core`'s `BigDecimal::div_round`, uniformly for terminating and repeating
  operands (`1/3 → 33/100`, `2.54 → 2.54`), with **no `f64` hop** deciding a tie.
  The default mode is round-half-even (`2.5 → 2`, not `3`). Dimension-preserving,
  like the unary round family. The `f64` result is derived from the exact value, so
  the labeled-lossy export and the exact audit value never disagree.

## [0.44.0] — 2026-07-21 — `verify`: re-execute a proof instead of believing it (RS-4 PR-D2)

Implements the checkability invariant of `ADJ-REASON-MATH.md` §E.5.

Every earlier PR in this arc made the audit trail *richer*. None made it
*checkable*. A richer trail nobody can re-run is still **testimony** — the engine
asserting what it did, in a format a confidently wrong system produces just as
fluently. This module turns testimony into **evidence**: it never reads the
trail's claims as authority, and instead goes back to the knowledge base and
does the work again.

### Added

- **`logic_engine::verify`** — `verify_proof(&Proof, &KnowledgeBase, &dyn SnapshotStore)`
  returns a `TraceVerification`: one verdict per step, in the proof's own
  preorder. Every one of the seven `DerivationOrigin` variants is re-executed:
  - `FromFact` / `FromRule` — the cited clause still exists and still unifies
    with the goal the step claims it proved.
  - `FromNegation` — the subgoal is **re-run** and must still have an empty
    proof set. A truncated search is a `NegationSearchTruncated` **failure**,
    not an absence: "I stopped looking" and "there is none" are different
    claims, and conflating them is the accounting failure this arc exists to
    prevent.
  - `FromPrior` / `FromContribution` / `FromJointContribution` — the clause is
    found by id, its evidence is re-observed, and `log(LR) × confidence` must
    reproduce the step's inline delta.
  - `FromPredicateContribution` — the slot is re-read and the comparison
    re-evaluated on CPU. The trail's own `observed` and `threshold` are the
    claim under test, never the inputs.
- **Two independent verdicts per step.** `LogicStatus` (did the inference go
  through?) and `QuoteStatus` (do the bytes say what it claims?) are reported
  separately. Collapsing them would lose the most interesting failure in the
  system: a *valid derivation from an invented fact*.
- **`QuoteStatus`, the five-valued outcome of §E.5** — `Verified`,
  `QuoteMissing` (the only status that fails a step), `Unverified`,
  `SourceDrifted`, `SourceUnreachable`, plus `NotApplicable` for negation steps,
  which rest on an absence and so have no sentence in any document. Separating
  drift and unreachability from "the quote is wrong" means a third party's
  outage — or a deliberate network denial — cannot invalidate a true trail.
- **Anchored quote checking.** The check requires a recorded byte offset and
  compares that exact range in the pinned snapshot. A span with no offset is
  `Unverified`, never verified-by-searching: on a long document a short phrase
  occurs *somewhere* with near-certainty, so an unanchored search would confirm
  the words exist, not that they support the clause. `byte_len` is reported
  alongside every verified span, because §E.3 declines to impose a minimum span
  length — false precision — so the honest alternative is to surface it.
- **`SnapshotStore`** (+ `NoSnapshots`, `MemorySnapshots`) — the seam through
  which the caller supplies snapshot *bytes*, since `Provenance` stores only a
  hash. A store that has nothing yields `Unverified(SnapshotUnavailable)`:
  honestly unchecked, never a pass.

### Security

- **The verifier re-checks the blank-span invariant itself** rather than trusting
  that a `VerbatimSpan` was built through its validating constructor. A blank or
  zero-width-only span is a substring of *every* document at *every* offset, so
  accepting one hands out `Verified` for free — and deserialization writes fields
  directly, running no constructor. The duplication of `is_invisible` between
  `provenance.rs` and `verify.rs` is deliberate: a check that only exists on the
  producer's side does not defend the consumer.
- **Every slice is bounds- and boundary-checked before it is taken.** An offset
  past the end, or inside a UTF-8 character, yields a verdict
  (`RangeOutOfBounds` / `NotACharBoundary`) rather than a panic. A verifier that
  panics on malformed input is a denial-of-service handed to whoever writes the
  trail.
- **No network access, by construction.** There is no HTTP client in this
  module. `locator`s are spider-authored strings from untrusted pages; fetching
  one would make the verifier an SSRF primitive aimed by anyone who can land a
  single KB entry. Live re-fetch belongs behind ADJ39's adapter registry.
- **An empty trace is not fully verified.** `all()` over nothing is `true`, and
  that vacuous truth would award the system's strongest verdict for having
  checked nothing.

## [0.43.0] — 2026-07-21 — the verbatim quote and its pinned snapshot (RS-4 PR-D1)

Implements `ADJ-REASON-MATH.md` §E.3 — the two fields that turn the audit trail
from something ADJ *reports* into something a third party can *check*.

### Added

- **`Provenance.quote: Quote`.** The verbatim span a clause rests on, separate
  from `source`. Until now the span was *stuffed into* `source` by convention,
  which conflates the **quotation** (bytes that must appear at the locator) with
  the **citation label** (how a human names the document). One string cannot be
  checked as both.
- **`Provenance.snapshot: Option<ContentHash>`.** A SHA-256 of the source
  document as captured at ingest. Verification runs against this, not the live
  web — a verbatim check against a live URL is decided by whoever controls that
  URL at verification time, so anyone able to publish there could make a
  fabricated quote verify. Pinning makes later divergence *evidence of drift*
  rather than a passing grade.
- `Quote::Verbatim { text, byte_offset }` records WHERE the span sits, so
  verification is **anchored** rather than an unanchored substring search. A
  search would confirm the words exist somewhere — in a footnote, a nav menu, or
  a passage saying the opposite — not that they support this clause.

- `Quote::Verbatim(VerbatimSpan)` — the payload's fields are **private**, with
  one fallible constructor. The invariant "a span must be able to support a
  claim" therefore holds on every construction path, not just inside a builder.

### Notes on two deliberate choices

- **`Quote` is an enum, not the `String` the spec literally writes.** A plain
  `String` cannot hold the `Unmigrated` state safely, and the obvious migration —
  defaulting `quote` to the `source` label — **fails open**: labels are short
  ("NIST", "AQI basics") and would trivially appear somewhere on the cited page,
  so the strongest check in the system would pass while checking nothing and
  report the step verified. A closed sum moves "never fail open" from a
  convention someone must remember into a fact the compiler enforces.
- **SHA-256, not the repo's `hash-functions` crate.** This hash is
  tamper-evidence, so it needs collision resistance; FNV/DJB2/murmur/SipHash have
  none and would look like a security control while providing nothing.
  `coding_adventures_sha256` is the repo's own zero-dependency implementation, so
  this stays inside the no-third-party rule.

### Compatibility

- Every existing `Provenance` constructor yields `Quote::Unmigrated` and
  `snapshot: None` — the honest record of "no checkable span was captured",
  never a guess. `adj-verify` (PR-D2) reports these `Unverified`, never
  `Verified`. No existing call site changed.

## [0.42.0] — 2026-07-21 — an empty result set now says WHY it is empty (RS-4 PR-C)

### Added

- **`ProofDAG.truncated`.** The search hit a resolution limit and gave up.
  Without it, "I found no proof" and "I stopped looking" were the SAME VALUE —
  an empty `proofs` — and they are completely different claims: the first is
  about the knowledge base, the second is about this run's budget and says
  nothing about the world. `ProofDAG::is_conclusively_empty()` is the predicate
  to use before asserting anything negative.
- **`GovernedResult::truncated()` and `conflict_status()` → `ConflictStatus`.**
  `has_conflict()` returns `false` both when it looked and found no tie and when
  it never finished looking. That second case was an affirmative claim ("no
  conflict among the answers") derived from an incomplete search. `ConflictStatus`
  makes `Unknown` a first-class third answer rather than an absence silently
  reported as a negative.

### Notes

- `lr_aggregate` sets `truncated: false` unconditionally and correctly: it walks
  a fixed clause list rather than searching, so it has no budget to exhaust.

All notable changes to this project will be documented in this file.

## [0.41.0] — 2026-07-20 — ordered, addressed proof steps + visible negation (RS-4 PR-B)

### Fixed (security)

- **Both resolvers had unbounded mutual recursion; a self-recursive rule aborted
  the process.** `solve`/`solve_body` (enumeration) and
  `find_first_with`/`prove_body` (deterministic) recursed with no termination
  guard, so `p(X, Y) :- p(X, Y)` descended until the stack overflowed —
  a `SIGABRT`, which cannot be caught, so a host embedding this crate dies with
  it. The deterministic path is the one `search(.., AutoDetect)` selects for an
  all-`Certain` KB, i.e. the adjudication connector's normal mode.
- **The obvious fix would have been worse than the crash.** Returning "no proof"
  at the cap is what negation-as-failure reads as *absence*, so a truncated
  search would have satisfied a `not G` guard and this release's new
  `FromNegation` step would have asserted a check that never happened. Both caps
  therefore raise `ResolutionLimitExceeded` and **propagate**; the `?` in each
  negation branch is load-bearing.
- **`MAX_SLD_DEPTH = 128`** bounds rule-chain nesting. **`MAX_BODY_CONJUNCTS = 1024`**
  bounds a separate axis the depth cap cannot: `solve_body` recurses over a
  rule's *remaining literals* while `depth` stays constant across the body, so a
  ~14,000-conjunct body overflowed at depth 1.
- A query that hits either cap **abstains** rather than reporting the proofs
  found first — a truncated search presented as a complete one is the accounting
  failure this release exists to prevent.


### Added

- **`ProofStep.depth`.** A step now records how deeply nested it is: the root
  query sits at depth 0, and a rule's body steps are one deeper than the rule
  step that introduced them. `Proof.steps` was already a preorder walk, so
  preorder + depth is a complete encoding of the derivation tree — a step's
  parent is the nearest preceding step one level shallower, exactly the way an
  indented outline works. Without it the flat vector was ambiguous: you could
  not tell a sibling from a child without re-deriving each rule's body arity,
  which is why the audit trail could show a LIST but never a STRUCTURE.
- **`DerivationOrigin::FromNegation { goal }`.** Negation-as-failure now records
  a step. It previously recorded **none**, so a rule guarded by
  `not contraindicated(D)` would fire while the trail stayed silent about the
  check that licensed it — a reader could not distinguish "we confirmed no
  contraindication" from "nobody looked." An audit trail that omits a
  load-bearing inference is not a shorter trail, it is a wrong one. The step
  carries no clause id because there is no clause: the justification IS the
  empty proof set, which is what a re-checker re-runs to verify it.

### Changed

- `collect_ids` handles `FromNegation` by contributing **nothing** — deliberately.
  NAF *used* nothing; that is precisely what it established. Attributing the
  absent goal's clauses as support would invert the meaning of the step.


## [0.40.0] — 2026-07-14 — exact rendering of a computed result (ADJ-EXACT-NUMBERS NX-4)

### Added

- **`ExactRational::to_exact_decimal_string(&self) -> Option<String>`** — the rendering side of the
  exact-numbers arc. When the exact-rational result of a computation has a finite base-10 expansion
  it returns all its digits (`3/4 → "0.75"`; a stored 39-digit π doubled →
  `"6.283185307179586476925286766559005768394"`); for a repeating expansion (`1/3`) it returns
  `None`, leaving the caller to fall back to the labeled-lossy `f64` from `to_f64()`. Delegates to
  the new `BigDecimal::from_rational_exact`. This is the compute-result analogue of NX-2's
  `Number::Exact` recall rendering: exact by default, `f64` only as a labeled fallback.
- Compute-layer tests: the doubled-π sidecar renders every digit and is strictly richer than its
  `f64` form; a repeating quotient (`1/3`) renders `None` while a terminating one (`3/4`) renders
  `"0.75"`.

## [0.39.0] — 2026-07-14 — exact compute ingestion of `Number::Exact` (ADJ-EXACT-NUMBERS NX-3)

### Changed

- **`numeric_exact_magnitude` now ingests a `Number::Exact(BigDecimal)` leaf at full precision**,
  replacing the NX-2 stopgap that folded the decimal through `to_f64()` (losing everything past the
  ~16th significant digit). A `BigDecimal` is `mantissa × 10^(-scale)`, an exact ratio, so the
  exact-rational sidecar is now populated via the new `BigDecimal::to_rational()` →
  `ExactRational::from_ratio(...)` — **no `f64` hop**. `Int` was already exact; `Float` remains the
  single documented inexact ingress. Net effect: `pi + pi` on the stdlib's stored 39-digit π stays
  exact to all digits (`6.283185307179586476925286766559005768394`) instead of collapsing to the
  f64-rounded `6.283185307179586`, and further arithmetic stays exact (NUM-5). Both the bare and
  typed-wrapper (`Compound`) arms were migrated. No result changes for `Int`/`Float` inputs.
- Added a compute-layer regression test proving the doubled-π sidecar equals the exact 40-digit
  value and reduces to `3141592653589793238462643383279502884197 / (5 × 10^38)` in lowest terms.

## [0.38.1] — 2026-07-14 — read `Number::Exact` valued facts (ADJ-EXACT-NUMBERS NX-2)

### Changed

- The compute-layer valued-fact readers now recognize the `Number::Exact` variant that `adj-lang`
  begins producing in NX-2. `numeric_magnitude`, `numeric_exact_magnitude`, `dimensioned_value`,
  and the `datetime` integral reader each gained an `Exact` arm that **folds into the existing
  `Float` handling via the labeled-lossy `to_f64` boundary** — so a valued fact stored exactly is
  visible to a formula/predicate exactly as it was when it stored an `f64`, and no compute result
  changes. (Before this, an `Exact`-valued slot was invisible and surfaced as `UnknownSlot`.)
  Ingesting an exact decimal's full precision into `ExactRational` **without** an `f64` hop is
  deliberately deferred to NX-3; this release only restores parity.

## [0.38.0] — exact arithmetic by default: `ExactRational` is now a `BigRational` (NUM-5)

### Changed

- **The compute engine's exact value is now arbitrary-precision.** `ExactRational` — the
  exactness carrier threaded through `compute`/`Derived`/the LR gate — was a pair of `i128`s
  that silently dropped to `None` on overflow; it is now a thin wrapper over
  `bignum_core::BigRational`, so `+ − × ÷` of rationals stay **exact and unbounded** — `1/3`
  is `1/3` past `i128`, `0.1 + 0.2` is exactly `3/10`, and the CSF:serum-style ratios never
  lose a digit. Overflow → `None` is gone; the only remaining `None`s are the genuinely
  non-rational cases (transcendentals, a fractional exponent, `gcd`/`lcm`/`mod`,
  aggregations). The `f64` magnitude on `Derived`/`DerivationNode` is unchanged for now — it
  is the **labeled lossy export** (`ExactRational::to_f64`), no longer the ground truth.
- Exact integer rounding (`Abs`/`Floor`/`Ceil`/`Round` ties-away/`Trunc`/`Sign`) is
  reconstructed from `BigInteger::div_rem`; exact ordering in the LR gate uses `BigRational`'s
  native total order (the old `i128` cross-multiply and its `f64` overflow fallback are gone —
  comparison is now always exact). Integer powers use `BigRational::try_pow` with a
  result-size guard (`MAX_EXACT_POW_BITS`) replacing the old bounded multiply loop.
- `MAX_EVAL_DEPTH` lowered `256 → 128`: each recursive `eval` frame now carries a heap-backed
  `BigRational` rather than two `i128`s, so the "clean `TooDeep`, never a stack overflow"
  guarantee needs a shallower cap on small (spawned-test-thread) stacks. 128 is still far
  deeper than any real formula nests.
- **Public JSON (adj-lang-cli):** the `exact` object's `num`/`den` are now emitted as JSON
  **strings** (`"exact":{"num":"3","den":"10"}`) since an arbitrary-precision numerator/
  denominator can exceed JSON's safe integer range.
- New dependency: `bignum-core` (zero-dependency, `#![forbid(unsafe_code)]`).

## [Unreleased] — optional provenance on `Derived`

### Added

- `Derived::provenance: Option<Provenance>` + `Derived::with_provenance(..)` — a
  computed value may now carry the cited `source`/`locator`/`trust` of the
  **formula** that produced it (ADJ-FORMULA-LIBRARIES rung-0 formula application).
  `compute` sets it to `None`; a plain `let` leaves it `None` (its audit trail is
  the derivation tree over observed facts). This is the channel by which a computed
  answer carries *why* its formula is trustworthy, so an independent checker can
  re-verify the citation without the model.

## [0.37.0] - 2026-07-02 — sign function (`Sign`)

### Added

- `ComputeOp::Sign` — the **mathematical** sign `sgn(x)`: `−1`/`0`/`+1` for a
  negative/zero/positive operand (NOT `f64::signum`, which returns `±1` for zero).
  A unary op folded into the **existing** `eval_unary` arm (no new recursion frame),
  but dimensionally in a category of its own: a sign is a pure number, so `sgn`
  **accepts any input dimension and collapses the result to `Scalar`** — unlike the
  dimension-preserving rounding family (`|dollars| = dollars`) and unlike the
  transcendentals (which reject a dimensioned operand). This makes the sign of a
  dimensioned difference (`sgn(pressure_a − pressure_b)`, a net pressure/charge/trend
  direction) a clean ±1. The result is exact (`±1`/`0` is rational), so the exact
  sidecar is the sign of the numerator carried as `q/1`. A NaN operand is produced
  explicitly so the shared non-finite guard rejects it rather than laundering it to
  `0`. `symbol()` renders it `"sgn"`. Lowered from a LaTeX `\operatorname{sgn}(x)`
  (adj-lang's `latex "…"` surface, operator-name juxtaposition like
  `\operatorname{trunc}`). +4 unit tests (pos/neg/zero scalar,
  dimensioned-operand-collapses-to-scalar, exact-sign sidecar, sign of a net
  difference).

## [0.36.0] - 2026-07-02 — binary modulo (`Mod`)

### Added

- `ComputeOp::Mod` — binary modulo `a mod b`, the remainder carrying the **sign of
  the dividend** (`7 mod 3 = 1`, `−7 mod 3 = −1`, `7.5 mod 2 = 1.5`), matching Rust's
  `f64::%` (truncated division / C `fmod`). Folded into the **existing** general binary
  `eval` arm next to `Div` (a single inline expression — no extra locals on the
  deeply-recursive path, preserving the clean-`TooDeep`-never-overflow contract):
  dimensionally it combines like addition (both operands must share a dimension and the
  remainder carries it — `7 mmol mod 3 mmol = 1 mmol`, while `7 mmol mod 3` is a category
  error), a zero divisor is a clean `DivisionByZero` (never a `NaN`), and — unlike
  `Gcd`/`Lcm` — it does **not** require integer operands. The exact-rational sidecar is
  dropped (the `f64` remainder already carries the value). This makes a LaTeX
  `a \bmod b` / `a \pmod{b}` (adj-lang's `latex "…"` surface) computable as a single
  native node. `symbol()` renders it `"mod"`. +4 unit tests (remainder + dimension
  preservation, sign-of-dividend / real operands, zero-divisor error, dimension
  mismatch).

## [0.35.0] - 2026-07-02 — truncation toward zero (`Trunc`)

### Added

- Unary compute op **`ComputeOp::Trunc`** — `trunc(x)` drops the fractional
  part toward zero (`trunc(3.7) = 3`, `trunc(−3.7) = −3`), completing the
  rounding family beside `Abs`/`Floor`/`Ceil`/`Round`. It is **dimension-
  preserving** (`trunc(3.7 mmol) = 3 mmol`) — folded into the existing
  `eval_unary` match arms (the f64 `value.trunc()` and the exact `num / den`,
  Rust integer division toward zero for `den > 0`), so `eval`'s recursive frame
  is unchanged. Contrast `Floor`, which rounds toward −∞ (`⌊−3.7⌋ = −4`): they
  agree only for a non-negative operand.
- Lowered from a LaTeX `\operatorname{trunc}(x)` on adj-lang's `latex "…"`
  surface (the operator-name juxtaposition path).

### Tests

- Two unit tests: `trunc(7/2) = 3` (dimension-preserving), and `trunc(−7/2) =
  −3` (toward zero, NOT the `Floor` −4).

## [0.34.0] - 2026-07-02 — binary gcd/lcm (`Gcd`/`Lcm`) integer ops

### Added

- Two binary compute ops **`ComputeOp::Gcd`** / **`ComputeOp::Lcm`** —
  `gcd(a, b)` / `lcm(a, b)` over exactly TWO operands, from a LaTeX
  `\gcd(a, b)` / `\lcm(a, b)`. They reuse the `Min2`/`Max2` binary-`Call` path
  (folded into the general binary arm + `dim_op`, so `eval`'s recursive frame is
  unchanged), but are **integer number-theoretic**: both operands must be exact
  integers in the exactly-representable range (`|v| ≤ 2^53`); a non-integer
  (`2.5`), NaN/inf, or out-of-range value is a clean `MalformedExpr`, never a
  silent truncation.
- Value computed by a leaf `#[inline(never)] int_gcd_lcm` helper (NOT on the
  recursion path, so its loop locals don't enlarge the recursive frame): `gcd`
  is Euclid on the magnitudes (`gcd(0, 0) = 0`, `gcd(n, 0) = |n|`), `lcm(a, b) =
  (|a| / gcd) · |b|` in `i128` (`lcm(_, 0) = 0`), with the shared `is_finite`
  guard as backstop. Dimensionally they combine like addition (a bare
  dimensionless integer stays one). Exact-rational sidecar dropped — the f64 is
  exact for the in-range integer results. `symbol()` renders `gcd` / `lcm`.

### Tests

- Three engine tests: gcd(12,18)=6 / lcm(4,6)=12, the zero edge cases
  (`gcd(0,0)`, `gcd(n,0)`, `lcm(_,0)`), and non-integer-operand rejection.

## [0.33.0] - 2026-07-02 — binary min/max (`Min2`/`Max2`), the first binary-Call op

### Added

- Two binary compute ops **`ComputeOp::Min2`** / **`ComputeOp::Max2`** —
  `min(a, b)` / `max(a, b)` over exactly TWO operands, carried in the existing
  `ComputeExpr::Bin` (the first binary-`Call` lowering, from a LaTeX
  `\min(a, b)` / `\max(a, b)`). Distinct from the slot-reducing aggregation
  `Min`/`Max` (which fold *every* observation of one slot).
- Evaluation is a **selection, not a combine**: dimensionally they behave like
  addition (both operands must share a dimension — `min(usd, days)` is the same
  `DimensionMismatch` category error as `usd + days` — and the result carries
  that shared dimension), the value is one operand chosen unchanged (ties pick
  the left), and the exact-rational sidecar is preserved verbatim from the
  winning operand (no rounding, no arithmetic). A non-finite operand is a clean
  `NonFinite` error rather than letting a NaN silently win a comparison.
- `symbol()` renders both as `min` / `max` for audit.

### Tests

- Four engine tests: extreme-operand selection (with a two-operand `Op` node
  shape assertion), exact-rational preservation of the winner, shared-dimension
  carry + mismatch rejection (`min(usd, usd)` vs `min(usd, days)`), and
  non-finite-operand rejection.

## [0.32.0] - 2026-07-02 — the rest of the trig family (inverse / hyperbolic / reciprocal)

### Added

- Nine more transcendental unary ops completing the standard trig set the LaTeX
  frontend already parses: **inverse** `Asin`/`Acos`/`Atan`, **hyperbolic**
  `Sinh`/`Cosh`/`Tanh`, and **reciprocal** `Cot` (cos/sin), `Sec` (1/cos), `Csc`
  (1/sin). Carried in the existing `ComputeExpr::Unary`, same contract as the
  earlier transcendentals: `Scalar → Scalar` (a dimensioned operand is a rejected
  category error), exact-rational sidecar dropped, evaluated in the shared
  `#[inline(never)] eval_unary` helper.
- Domain and pole errors go through the non-finite guard rather than a
  silently-wrong number: `asin`/`acos` outside `[−1, 1]` → `NaN`; `cot`/`csc` at a
  multiple of π and `sec` at an odd multiple of π/2 → `±∞` (the reciprocal
  definitions divide by a zero primary).

## [0.31.0] - 2026-07-02 — native transcendental functions (`sin`/`cos`/`tan`/`ln`/`log`/`exp`)

### Added

- Six **named transcendental** unary ops — `ComputeOp::Sin`, `Cos`, `Tan`, `Ln`
  (natural log), `Log` (base-10), `Exp` — carried in the existing
  `ComputeExpr::Unary` (no new node type) and evaluated in the shared
  `#[inline(never)] eval_unary` helper (the macOS stack-frame guard still holds).
  They make a LaTeX `\sin(x)` / `\ln(x)` / `\exp(x)` … named-function call
  (adj-lang's `latex "…"` surface) computable as a single native node.
- Unlike the rounding unary ops these are **not** dimension-preserving: a
  transcendental is only defined on a pure number, so the operand must be
  dimensionless (`Scalar`) and the result is `Scalar`. `sin(3 dollars)` is a
  category error, rejected with the same `DimensionMismatch` the binary ops raise
  (operand dimension vs the required `Scalar`). They are irrational in general, so
  they drop the exact-rational sidecar.
- Domain errors surface through the existing non-finite guard rather than flowing
  a bad value into a verdict: `ln` of a non-positive number (`−∞`/`NaN`), `exp`
  overflow (`+∞`), `tan` near a pole — all become a clean `NonFinite` error.

## [0.30.0] - 2026-07-02 — native round-to-nearest (`ComputeOp::Round`)

### Added

- **`ComputeOp::Round`** (`⌊x⌉`, nearest integer, **ties away from zero**) — a
  third rounding unary op alongside `Floor`/`Ceil`, carried in the existing
  `ComputeExpr::Unary` and evaluated in the shared `#[inline(never)] eval_unary`
  helper (the macOS stack-frame guard still holds). It makes the standard
  nearest-integer LaTeX fence `\left\lfloor x\right\rceil` (adj-lang's `latex "…"`
  surface) computable as a single native node.
- Dimension-preserving like `Floor`/`Ceil` (`⌊3.6 mmol⌉ = 4 mmol`). The **exact
  rational sidecar stays exact**: truncate toward zero, then bump one step outward
  when the fractional part reaches a half (`2·|rem| ≥ den`), matching Rust's
  `f64::round` — `⌊5/2⌉ = 3`, `⌊−5/2⌉ = −3`, `⌊7/3⌉ = 2`. The `den − arem` compare
  (with `arem = |rem| < den`) sidesteps the overflow a bare `2·arem` could hit, and
  the outward bump is a `checked_add` (drops the exact sidecar rather than
  panicking on the i128 edge).

## [0.29.0] - 2026-07-02 — native floor & ceiling (`ComputeOp::Floor` / `ComputeOp::Ceil`)

### Added

- **`ComputeOp::Floor`** (`⌊x⌋`, greatest integer ≤ x) and **`ComputeOp::Ceil`**
  (`⌈x⌉`, least integer ≥ x) — two more **unary**, **dimension-preserving** ops
  carried in the existing `ComputeExpr::Unary`, mirroring `Abs`. They make a
  LaTeX `\left\lfloor x\right\rfloor` / `\left\lceil x\right\rceil` (adj-lang's
  `latex "…"` surface) computable as a single native node.
- Like `Abs`, both are dimension-preserving — the magnitude snaps to an integer
  but the **unit does not** (`⌊3.7 mmol⌋ = 3 mmol`, `⌈3.2 mmol⌉ = 4 mmol`) — so
  the operand's dimension flows straight through, and they reuse the n-ary
  `DerivationNode::Op` node (one operand): no new audit-tree variant.
- The **exact rational sidecar stays exact**: `ExactRational` keeps `den > 0`, so
  `⌊num/den⌋ = num.div_euclid(den)` (Euclidean division floors) and
  `⌈num/den⌉` adds one only when the division leaves a remainder — each result an
  integer `q/1`. Floor rounds toward −∞ (`⌊−7/2⌋ = −4`, NOT −3).
- Evaluated in the shared `#[inline(never)] eval_unary` helper, so the deeply
  recursive `eval` frame stays small (the macOS stack-overflow guard from the
  `Abs` slice still holds).

## [0.28.0] - 2026-07-01 — native absolute value (`ComputeOp::Abs`)

### Added

- **`ComputeOp::Abs`** — a native **unary**, **dimension-preserving** absolute
  value, carried in a new `ComputeExpr::Unary(ComputeOp, Box<ComputeExpr>)`. It
  is what makes a LaTeX `|x|`/`\left|x\right|` (adj-lang's `latex "…"` surface)
  computable instead of being silently dropped to a bare `x`. `|−7| = 7`; the
  exact rational sidecar stays exact (`|−7/1| = 7/1`).
- Unlike `Pow`, `Abs` neither combines two dimensions nor collapses to `Scalar`:
  a magnitude flips sign but the **unit does not** (`|−4 dollars| = 4 dollars`),
  so the operand's dimension flows straight through. It reuses the existing
  n-ary `DerivationNode::Op` node (one operand) — no new audit-tree variant.
- A finite operand yields a finite result; the arm re-checks `is_finite()` as
  defense-in-depth (same "no silently-wrong number" contract as the binary ops).

## [0.27.0] - 2026-07-01 — native `^` power operator (`ComputeOp::Pow`)

### Added

- `ComputeOp::Pow` — a native exponentiation operator, `base ^ exponent`, in the
  `compute` formula IR. Unlike the additive/multiplicative ops it is **not** a
  symmetric dimensional combine: the exponent must be dimensionless (`Scalar`),
  and the result dimension is the *base* raised to the exponent — `x^0 = Scalar`
  (dimensionless), `x^1 = x`, `x^2 = x·x` — computed by folding through the
  multiplicative algebra so it matches an expanded `x*x*…` chain exactly, but as
  a single derivation node (audit symbol `^`). This is what makes a LaTeX `x^n`
  (adj-lang's `latex "…"` surface) computable as one node rather than the
  parse-time repeated-multiplication expansion the adapter used, and lays the
  groundwork for lifting that expansion's integer-exponent cap.
- `Dimension::pow(exponent)` — the dimensional rule for a power: a scalar base is
  closed under any exponent; a dimensioned base needs a non-negative integer
  exponent (then it folds `x·x·…`), and a fractional/negative power of a
  dimensioned base (a √dollars, a 1/dollars) is a `DimError::Mismatch` rather
  than a silently-wrong tag. `Date` bases are rejected (reuses `combine`).
- `ExactRational::powi(exp)` — exact non-negative-integer powers by repeated
  multiplication so `(3/2)^2 = 9/4` keeps its exact sidecar. Bounded by
  `MAX_EXACT_POW = 1024` (an algorithmic-DoS guard: a pathological
  `base^{10^18}` can't spin the loop — the `f64` magnitude stays authoritative).

### Notes

- Purely additive. `ComputeOp::Pow` overflowing `f64` (`10^400`) is a clean
  `ComputeError::NonFinite`, never a silent `inf`. Downstream consumers that
  match `ComputeOp` (the `adj-constraint-solver` linear/polynomial recognisers)
  already have catch-all arms, so a `Pow` term is treated as non-linear (→
  `Unknown`) without any change on their side.

## [0.26.0] - 2026-06-29 — expose derived dimensioned bindings

### Added

- `KnowledgeBase::derived_bindings()` — a read-only view of every `let`-bound
  `compute::Derived` value (name, magnitude, exact rational, and the `Dimension`
  the engine inferred), in binding order. The engine already carried dimensions
  through each `let` via `Dimension::combine`; this accessor surfaces them so a
  consumer (the `adj-lang-cli` JSON renderer, a UI, a proof checker) can audit
  the dimensional analysis — e.g. report `240 km / 3 h` as `80 km/h`, not just
  `80`. Purely additive; `derived_for` keeps the latest-wins lookup rule.

## [0.25.0] - 2026-06-28 — rule-derived evidence gates LR contributions

### Added

- `KnowledgeBase::observed_evidence` now falls back to SLD proof enumeration when
  an LR evidence term is not directly asserted as a `Certain` fact. A derived atom
  such as `infection_present` can now fire `contributes ... from infection_present`
  if rules prove it from observed case facts.
- `ObservedEvidence` records the direct fact ids, derived rule ids, optional SLD
  proof, and confidence factor for the evidence gate.
- LR aggregation attenuates a contribution's applied logit delta by the selected
  proof's confidence, computed as the product of fact/rule probabilities along the
  proof. Certain proof chains remain fully backward-compatible with confidence 1.0.
- `DerivationOrigin::FromContribution` and `FromJointContribution` can carry nested
  evidence proofs, so aggregate proofs expose the rule/fact chain that licensed a
  probabilistic step.

## [0.24.0] - 2026-06-27 — exact rational predicate comparisons

### Added

- `compute::ExactRational` sidecars for integer/rational arithmetic. `compute`
  still exposes `f64` magnitudes for compatibility, but derived values now keep
  an exact value when literals, references, and binary arithmetic stay inside
  exact integer/rational operations.
- `PredicateContributionClause::from_lr_expr` and `CmpOp::eval_values`, allowing
  predicate gates to compare against a computed right-hand expression and use
  exact rational equality when both sides carry exact values.
- `KnowledgeBase::observed_numeric` and `observed_exact_value_with_fact` so
  predicate gates can read observed or derived magnitudes with their exact
  sidecars when available.

## [0.23.0] - 2026-06-21 — multi-source corroboration on `Provenance` (ADJ-A9)

### Added

- **`Citation` { `source`, `locator` }** — a re-fetchable corroborating citation
  (both fields required).
- **`Provenance::corroborations: Vec<Citation>`** — co-equal citations that
  support the *same* clause/LR. Distinct from `source_disagreements`, which
  compares *different* clauses whose LRs disagree; these are documentary only
  and carry **no** evidential weight (the LR arithmetic is unchanged — double-
  counting the same fact would inflate posteriors). Builder
  `Provenance::with_corroboration(source, locator)` appends one.

### Compatibility

- Fully additive: the new field defaults to empty in every constructor
  (`new`, `cited`, `consensus`, `empirical`, `unattributed`, `Default`); all
  existing callers and downstream consumers (`adj-lang`, `adj-constraint-solver`,
  `adjudication-connector`, `adjudication-pipeline`, `prolog-loader`) compile and
  pass unchanged.

## [0.22.0] - 2026-06-17 — mutual precedence is an honest CONFLICT, not silent double-defeat (ADJ73 §4.3)

### Fixed

- **Contradictory precedence between two competing answers now surfaces as `ConflictPeer`
  (abstain), not a silent "both defeated, `has_conflict == false`".** When two canons point
  opposite ways — e.g. lex superior derives `federal > state` while lex specialis derives
  `state > federal` — each answer defeated the other, so the resolver marked BOTH `Defeated`,
  crowned nothing, and reported no conflict (misleading). The defeat test in `enumerate_governing`
  is now **strict domination**: `j` defeats `i` only if `j` defeats `i` AND `i` does not defeat
  `j` back. A merely mutual defeat leaves both answers undefeated → the group resolves to
  `ConflictPeer` / `has_conflict == true` — the honest "else CONFLICT (abstain)" the spec
  (§4.3) promises. The caller is never silently handed an empty governing set with no conflict
  signal.

### Unchanged (no regression)

- One-way precedence (the ordinary lex-superior / tier case) still cleanly governs: a strictly
  dominating answer wins, the dominated one is `Defeated { by }`. Co-equal-tier and cyclic-order
  cases already abstained and still do. The context order is a partial order + a total tier, so
  only 2-cycles of mutual defeat arise (transitivity makes any longer cycle mutual everywhere) —
  no strict Condorcet cycle can slip through. All prior `govern` tests pass unchanged.

## [0.21.0] - 2026-06-17 — context-precedence edges can be DERIVED by meta-rules (ADJ73 PR-B-4)

### Added

- **`outranks_context` edges may now be RULE-DERIVED, not just asserted.** When the KB contains
  rules whose head is `outranks_context/2` (the grounded conflict-resolution **meta-rules** — lex
  posterior / appeal status / lex specialis), `KnowledgeBase::context_adjacency` enumerates every
  provable `outranks_context($A, $B)` (via `enumerate_all`, which subsumes the ground facts as
  one-step proofs) and feeds those edges into the same `lex superior` resolution. So a meta-rule
  `outranks_context($H, $L) :- reverses($H, $L)` (itself citing the overruling doctrine) turns a
  primitive grounded `reverses(a, b)` fact into a precedence edge — the recursive structure ADJ73
  §7 calls for: an edge that can be derived is derived (and cited), not duplicated as a bare fact.
- New private `KnowledgeBase::derived_context_edges` — enumerates the provable `outranks_context`
  ground edges. Pure read; not re-entrant with `enumerate_governing` (queries a different predicate
  and never consults the context order itself).

### Unchanged (back-compat / performance)

- With **no** `outranks_context` rules (the common case), `context_adjacency` keeps the cheap
  ground-fact + explicit-edge scan — no SLD enumeration cost. All PR-B / PR-B-2 behavior is
  byte-identical; `context_outranks` / `context_order_has_cycle` now key on owned `String`s
  (derived answer terms are built during enumeration), but the DFS / Kahn-cycle semantics are
  unchanged. Cycle detection spans derived edges too (a contradictory `reverses` pair is caught).

## [0.20.0] - 2026-06-17 — context-precedence edges as grounded facts (ADJ73 PR-B-2)

### Added

- **Grounded `outranks_context(higher, lower)` facts now ACT as context-precedence edges.** A
  ground fact whose functor is `outranks_context` and whose two args are atoms participates in
  the context order exactly like an explicit `add_context_outranks` edge — but, being an ordinary
  [`Fact`], it carries `source`/`locator`/`trust` [`Provenance`], is queryable, and is one CAS
  edit from correctable. This is the mechanism behind ADJ73's "context must be grounded": the
  *reason* federal outranks state (the Supremacy Clause) rides on the edge itself rather than
  being asserted bare in host code. A `relate outranks_context(federal, state) source "…" trust
  authoritative` clause in adj-lang already lowers to such a fact — no surface change needed.
- **`KnowledgeBase::context_adjacency()`** (private) — the EFFECTIVE order as a directed
  adjacency map `higher → [lowers]`, unioning explicit `add_context_outranks` edges and
  grounded-fact edges. `context_outranks` is a cycle-safe DFS over it (O(V+E)); `context_order_has_cycle`
  is a single Kahn topological-sort pass (O(V+E)) — both detect a cycle formed *across* the two
  sources (explicit `a > b` + grounded `outranks_context(b, a)`). The per-node adjacency lookup
  replaces the prior full-edge-list rescan so the order scales to large rule corpora.

### Unchanged (back-compat)

- A KB with no `outranks_context` facts and no `add_context_outranks` calls is byte-identical to
  0.19 (every existing precedence + integration test passes unchanged). A non-atom
  `outranks_context(_, X)` fact is NOT an edge — it stays an ordinary queryable fact.

### Scope

PR-B-2. The grounded `context-precedence` **rulebook** (a `.adj` library of `relate
outranks_context(…)` edges, each byte-quoting its charter — the Supremacy Clause, circuit
precedence, guideline editions) + a worked legal example end-to-end through the CLI `governing`
section land in PR-B-3 (now unblocked by this engine change). The recursive conflict-resolution
meta-rules (recency / appeal-status / lex specialis as themselves-grounded rules) follow.

## [0.19.0] - 2026-06-16 — grounded context precedence (lex superior) (ADJ73 PR-B engine core)

### Added

- **`Rule::context: Option<String>`** (builder `with_context`) — the context a rule is grounded
  in (jurisdiction / guideline edition / specialty). `None` = context-free (today's behavior).
- **`KnowledgeBase::add_context_outranks(higher, lower)`** — assert a grounded precedence edge
  (federal > state, ninth_circuit > district_court, idsa_2024 > idsa_2004, specialist > general).
- **`KnowledgeBase::context_outranks(a, b)`** — transitive reach over the edges (cycle-safe DFS);
  **`context_order_has_cycle()`** — detect a cyclic order (the surface loader should reject).
- **`govern::GovernedAnswer::context`** + a `defeats(a, b)` resolution: context precedence is
  PRIMARY (lex superior — a higher-context answer defeats a lower-context one regardless of
  tier); the [`Standing`] tier breaks ties the context order leaves open. An answer governs iff
  no conflicting answer defeats it; multiple undefeated → `ConflictPeer`; a cyclic order crowns
  nothing (safe degradation, never a wrong pick).

### Unchanged (back-compat)

- With NO `context_order` declared, `defeats` reduces to "higher tier wins" — resolution is
  byte-identical to 0.18 (verified: the existing precedence + integration tests pass unchanged).
  The two `adjudication-connector` `Rule{}` sites set `context: None`.

### Scope

PR-B engine core. The grounded `context-precedence` **rulebook** (each `outranks_context` edge
citing its charter — the Supremacy Clause, etc.) + adj-lang **surface** (`context:` on a rule,
`context_order { … }`) are the next slices (ADJ73 §7). Cycle *rejection* at load is the loader's
job; the resolver itself stays safe regardless.

## [0.18.0] - 2026-06-16 — precedence priority is a named ENUM, not an integer (ADJ73 PR-A)

### Changed (breaking — nothing released; per user decision 1)

- **`Rule::priority` is now `Priority`** (a named enum) instead of `i64`. Tiers, totally
  ordered lowest→highest: `Default < Specific < Authoritative < Mandatory`. `Default` is the
  implicit tier (existing rules unchanged). `Rule::with_priority` now takes a `Priority`.
- **`GovernedAnswer::priority` is now `Standing`** (new enum): `Standing::Rule(Priority)` or
  `Standing::Asserted` (a ground fact — outranks every rule tier, replacing the old `i64::MAX`
  sentinel). `Standing` derives `Ord` (Asserted greatest), so the resolver compares tiers
  without magic numbers.
- The two `adjudication-connector` `Rule{}` literal sites set `priority: Priority::Default`.

### Rationale

Raw integers were magic-numbery; named tiers read correctly in grounded rulebooks and are the
simplest *grounded precedence principle* ("a higher tier wins"). Richer, byte-provenanced
precedence (a grounded `context-precedence` rulebook with lex-superior / recency / appeal-status
meta-rules) is ADJ73 PR-B; the recursive grounded design is now spec'd in
`code/specs/ADJ73-defeasible-rule-precedence.md` §2.3 + §7.

### Unchanged

- Resolution semantics, opt-in-per-predicate `declare_functional`, and back-compat of
  `enumerate_all` are exactly as in 0.17.0. All 101 + 5 (govern) + 4 (precedence integration)
  tests pass.

## [0.17.0] - 2026-06-16 — defeasible rule precedence (ADJ73 PR-1)

### Added

- **`Rule::priority: i64`** (default `0`, builder `Rule::with_priority(p)`) — a rule's
  precedence among *conflicting* derivations. Higher defeats lower.
- **`KnowledgeBase::declare_functional(functor, arity)`** — mark a predicate FUNCTIONAL on
  its last argument (at most one value per key = the preceding args). Two derivations that
  share the key but differ on the last argument *conflict*.
- **`govern::enumerate_governing(query, kb) -> GovernedResult`** — runs `enumerate_all`, then
  resolves conflicting answers by precedence as a post-pass: the unique maximum-priority answer
  in a conflict group **governs**; the rest are **`Defeated { by }`**; a tie at the maximum is
  surfaced as **`ConflictPeer`** (never silently resolved). A fact-derived answer has priority
  `i64::MAX` (asserted truth outranks any rule). `GovernedResult::governing()` /
  `has_conflict()` helpers.

### Unchanged (back-compat)

- `enumerate_all` and SLD search are **untouched**: a query over predicates none of which are
  declared functional returns every answer as `Governing` (today's semantics exactly).
  Precedence is opt-in per predicate. The new `Rule.priority` field defaults to `0`; the two
  `adjudication-connector` `Rule{}` literal sites set it explicitly.

### Scope

- PR-1 ships the **functional-predicate conflict relation + total integer priority**. Explicit
  `conflict {}` sets and the `context_order` partial order (ADJ73 §2, the legal-context
  precedence) are PR-1b — they reuse this same resolution post-pass. Surface syntax in adj-lang
  is PR-2. See `code/specs/ADJ73-defeasible-rule-precedence.md`.

## [0.16.0] - 2026-06-14 — `KnowledgeBase::fact(id)` accessor (MYCIN-2026 REL-3)

### Added

- **`KnowledgeBase::fact(&self, id: FactId) -> Option<&Fact>`** — resolve a
  proof's `via_facts` (or a `DerivationOrigin::FromFact`) back to the firing fact,
  in particular its `provenance`, so a relational recall binding query's answer
  can be returned WITH the citing edge's source.

## [0.15.0] - 2026-06-14 — mandatory `Fact::provenance` for relational edges (MYCIN-2026 REL-2)

### Added / Changed (breaking)

- **`Fact::provenance: Provenance`** (mandatory — every fact is accountable) +
  the `Fact::with_provenance(p)` builder. A ground relational edge (adj-lang's
  `relate` clause) lowers to a `Fact` that carries its citation, so a binding
  query's answer (`? deficient_in(tay_sachs, $E)` → `hexosaminidase_a`) is
  returned WITH a proof — the byte-provenanced source that justifies the edge.
  Ordinary `observe`d facts carry `Provenance::unattributed()` — the explicit
  "no source" value, not a silent `None`. **Breaking:** the field is `Provenance`,
  not `Option<Provenance>`; the two `Fact` builders default it to
  `Provenance::unattributed()`, so all existing construction sites compile
  unchanged, but any code matching `fact.provenance` as an `Option` must adapt.
  `add_fact` preserves it.

## [0.14.0] - 2026-06-11 — dimensional faithfulness gate (ADJ constraints track A4)

### Changed

- **`compute` is now dimension-aware.** Alongside the f64 magnitude, the
  evaluator tracks each value's `Dimension` (read from its fact via
  `dimensioned_value`) and checks every binary op through `Dimension::combine`,
  so a unit-mismatched formula — `usd + days`, `usd + eur` without a conversion
  — is a clean **`ComputeError::DimensionMismatch`** instead of a
  silently-wrong number. This is the faithfulness gate: the engine, not the
  model, decides a category error is a category error.
  - `usd + usd → usd`; `money / money → scalar` (a dimensionless ratio,
    e.g. debt-to-income); `money × scalar → money`; bare-number formulas stay
    `Scalar` (the pre-A4 numeric behaviour is unchanged).
- **`Derived` gains a `dim: Dimension`** field — the inferred dimension of the
  computed value, so a predicate firing over it (`csf_ratio <= 0.4`) knows
  `csf_ratio` is a `Scalar` and the audit shows the unit. (Additive: callers
  that pass a `Derived` through unchanged are unaffected.)

### Added

- `KnowledgeBase::observed_dimensioned(slot)` — the dimensioned (`magnitude +
  Dimension`) observation of a slot with its `FactId`, for the gate.
- `ComputeError::DimensionMismatch { op, lhs, rhs }` carrying the two clashing
  unit tags for the audit reader.

## [0.13.0] - 2026-06-11 — date arithmetic (deadlines & durations, ADJ constraints track A3)

### Added

- **`datetime` module** — calendar arithmetic on the CPU for adjudication
  deadlines ("is the claim within 365 days of purchase?"). A date is a *point
  in time*, so it gets the new `Dimension::Date` and its arithmetic lives here,
  not in the generic `Dimension::combine` (which now rejects any `Date`
  operand, steering callers to these functions):
  - `days_between(a, b)` → a `Duration("days")` dimensioned value (so a deadline
    predicate `elapsed <= 365` fires over it).
  - `date_add(date, days)` → the resulting `(y, m, d)` (`Date + Duration → Date`).
  - `before(a, b)` / `after(a, b)` → a boolean ordering.
  - `read_date` validates month (`1..=12`) and day (`1..=days_in_month`, leap-aware)
    so `date(2025, 13, 40)` is a clean `None`; `read_duration_days` reads
    `duration(n, days|weeks)`.
- `days_from_civil` / `civil_from_days` — Howard Hinnant's public-domain
  proleptic-Gregorian ↔ day-ordinal algorithm, **inlined** (not a dependency).
  The repo's `datetime-core` is the right library but pulls `numeric-tower` /
  `r-vector` / `wall-clock` — too heavy for the core engine; the algorithm is
  ~25 lines of exact integer math, so we inline it and keep `logic-engine`
  dependency-free.
- `Dimension::Date`; `dimensioned_value` now returns `None` for `date`/`time`/
  `datetime` terms (their leading field is a year, not a scalar magnitude).

### Security (from /security-review, both LOW, fixed in-PR)

- All ordinal arithmetic is overflow-safe on attacker-controlled fields: `read_date`
  bounds the year to `±1_000_000`; `date_add` uses `checked_add` + an ordinal
  bound; `read_duration_days` uses `checked_mul` + a bound; and the raw
  `days_from_civil`/`civil_from_days` helpers are now `pub(crate)` (internal),
  so the public surface (`days_between`/`date_add`/`before`/`after`) can't be
  handed an unbounded `i64` that would overflow.

Time-of-day and full datetime arithmetic are a follow-up; this slice is dates +
durations (the deadline case). See
`code/specs/data/adj-language-expansion/ADJ-CONSTRAINTS-DESIGN.md`.

## [0.12.0] - 2026-06-11 — currency conversions (ADJ constraints track A2)

### Added

- **`conversion` module** — the only thing that licenses a cross-currency
  operation (which A1 made a `DimError::Mismatch`): an **explicit, provenanced
  conversion fact**. `Conversion::new("usd", "eur", 0.92)` = "1 usd = 0.92 eur",
  carrying a `Provenance` citation. `ConversionTable::rate(from, to)` resolves a
  rate via a direct fact, its inverse (`1/rate`), or the identity; no transitive
  chaining (a missing path is a clean `None`, never a guess).
- `convert_value(value, target, table)` converts a `Dimensioned` between
  currencies/units (money→money, unit→unit), re-tagging the dimension;
  `Scalar`/`Percent` are not convertible.
- `add_or_sub(subtract, lhs, rhs, table)` — dimension-aware add/sub that resolves
  a currency mismatch by converting `rhs` into `lhs`'s dimension via the rate
  (so `100 usd + 92 eur` = `200 usd` given `1 usd = 0.92 eur`), and still rejects
  genuinely incompatible kinds (`usd + days`). `ConvError::{NoRate, NotConvertible}`.
- `Conversion::try_new` validates the rate and returns `ConvError::BadRate`
  for a non-finite/non-positive value (the entry point a surface-`convert`
  lowerer should call, mirroring the LR/probability guards); `new` is the
  panicking trusted/test convenience. `convert_value`/`add_or_sub` screen for a
  non-finite result (`ConvError::NonFinite`), matching `ComputeError::NonFinite`
  so a converted value can't silently flow non-finite into a verdict.

Engine-only; the surface `convert money(1,usd) = money(0.92,eur)` statement and
recording the rate as a derivation-tree `Op` land with the constraint
sublanguage (B1) and the dimensional faithfulness gate (A4). See
`code/specs/data/adj-language-expansion/ADJ-CONSTRAINTS-DESIGN.md`.

## [0.11.0] - 2026-06-11 — dimensional types (strict units, ADJ constraints track A1)

### Added

- **`dimension` module** — every value gets a `Dimension`
  (`Scalar`/`Money(ccy)`/`Unit(tag)`/`Percent`/`Duration(unit)`) so the engine,
  not the model, decides which operations are category errors. `Dimension::combine(op, l, r)`
  encodes the strict algebra: **add/sub require matching dimensions** (`usd + eur`
  and `usd + days` are rejected — `usd + eur` will need a conversion fact in
  track A2); **`Money/Money → Scalar`** and `Unit(a)/Unit(a) → Scalar` (units
  cancel — the CSF:serum/debt-to-income ratio is dimensionless); `Money × Scalar
  → Money`, `× Percent` keeps the dimension; unlike dimensions multiply/divide to
  a composite tag the faithfulness gate can inspect.
- **`dimensioned_value(&Term)`** — generalises `numeric_magnitude` (step 2):
  reads the leading magnitude **and** infers the dimension from the wrapper
  functor (`money(18000, usd)` → `Money("usd")`, `quantity(40, mg_dl)` →
  `Unit("mg_dl")`, …). Tags are compared by equality, never interpreted (the
  engine knows `usd ≠ eur`, not that usd is dollars).
- `DimOp`, `DimError::Mismatch`, `Dimensioned`. This is the foundation for
  currency/date arithmetic (A2/A3) and the dimensional faithfulness gate (A4);
  `compute` stays numeric until A4 wires this in. See
  `code/specs/data/adj-language-expansion/ADJ-CONSTRAINTS-DESIGN.md`.

## [0.10.0] - 2026-06-11 — derivation tree (provenance-through-math, ADJ expansion step 3a)

### Added

- **`compute` module — the engine half of "the model never does the math".**
  A formula IR (`ComputeExpr`: `Ref(slot)` / `Lit(n)` / `Bin(op,a,b)` /
  `Agg(op,slot)`) is evaluated deterministically on the CPU into a `Derived`
  value carrying a **derivation tree** (`DerivationNode`): every operation
  records its operands and result, and every leaf cites the `FactId` of the
  observed fact it came from. So a derived value (`csf_ratio = csf_glucose /
  serum_glucose = 0.4`) is fully reconstructable from the tree without the
  model — provenance-through-math.
- `ComputeOp` — `Add/Sub/Mul/Div` (binary) and `Sum/Count/Min/Max/Avg`
  (aggregation over every observation of a slot). Operands read the magnitude
  of typed values (`quantity(40, mg_dl)`) via `numeric_magnitude`.
- `ComputeError` — clean, non-panicking errors (`UnknownSlot`,
  `EmptyAggregation`, `DivisionByZero`, `MalformedExpr`, plus two safety
  guards: `TooDeep` bounds recursion at `MAX_EVAL_DEPTH` so an adversarially
  deep formula returns an error instead of overflowing the stack, and
  `NonFinite` rejects any `NaN`/`±∞` result rather than letting it silently
  flow into a verdict — a `NaN` compares `false` against every threshold, so
  an unscreened non-finite would quietly make a predicate not fire).
- `KnowledgeBase::add_derived` / `derived_for`; `observed_value(slot)` now
  falls back to the derived table, so a **predicate-gated contribution fires
  over a computed value exactly as over an observed one** — one engine, no new
  verdict logic. New helpers `observed_value_with_fact` /
  `observed_values_all` expose the `FactId`(s) the derivation-tree leaves cite.
- A derived value can reference a previously-bound derived value (`let` over
  `let`) via a `DerivationNode::DerivedRef`.

This is engine-only (no surface syntax yet — `let name = expr` is step 3b). See
`code/specs/data/adj-language-expansion/STEP3-let-arithmetic-PLAN.md`.

## [0.9.0] - 2026-06-10 — typed-value magnitudes (ADJ language expansion, step 2)

### Added

- **`numeric_magnitude(&Term) -> Option<f64>`** — extract the numeric
  magnitude of a typed value. The ADJ language expansion models a fact's
  value as either a bare number or a *typed-value wrapper* carrying the
  magnitude as its leading argument and the unit afterward:
  `quantity(18000, usd)`, `money(18000, usd)`, `percentage(40)`,
  `duration(365, days)`, `count(3)`. The rule is uniform — "the leading
  numeric argument" — so no closed set of wrapper functors is hard-coded.

### Changed

- **`observed_value(slot)`** now reads through a typed-value wrapper via
  `numeric_magnitude`, so a predicate (`gross_income >= 14600`) fires over
  `observe gross_income(quantity(18000, usd))` while the `usd` unit stays
  attached to the fact for the (forthcoming) faithfulness gate. Bare
  `slot(Num)` facts behave exactly as before.

## [0.8.0] - 2026-06-10 — predicate-gated contributions (deterministic = saturating probabilistic)

### Added

- **`PredicateContributionClause` + `CmpOp`** — a likelihood-ratio
  contribution gated by a numeric comparison over a *valued slot*:
  "when the observed value of `slot` satisfies `slot <op> value`,
  multiply the conclusion's odds by `exp(logit_delta)`." This is the
  bridge that lets the framework express a **deterministic** rule as
  the saturating limit of a probabilistic one — a hard rule is just a
  very large LR over a CPU-evaluated predicate. DETERMINATE /
  INDETERMINATE / CONFLICT continue to fall out of the existing
  `differential` (leader / insufficient-evidence / kickback); there is
  **no second engine**.
- `CmpOp` — `Ge` / `Le` / `Gt` / `Lt` / `Eq` with `eval(lhs, rhs)`
  (the comparison the engine runs on the CPU) and `symbol()` (for
  audit rendering). `Eq` uses an absolute tolerance so an integer
  observation matches a float threshold.
- `KnowledgeBase::add_predicate_contribution`,
  `predicate_contributions_for`, and `observed_value(slot)` — the
  last reads the numeric value of the latest `Certain` valued fact
  `slot(V)` (V a `Term::Num`). Predicate clauses also count toward
  `participates_in_lr_aggregation`.
- `DerivationOrigin::FromPredicateContribution { clause_id, slot, op,
  threshold, observed, logit_delta }` — the proof step records the
  *literal* comparison that fired, so the audit trail shows the
  numbers the engine compared. The model never computes the
  comparison; it only authored the rule.

## [0.7.0] - 2026-06-10

### Added

- **`differential(hypotheses, kb)` — the cross-hypothesis decision
  primitive.** `lr_aggregate` scores one hypothesis at a time; the
  differential ranks a set of *competing* hypotheses (bacterial vs
  viral vs fungal meningitis, charge A vs B, deal-vs-no-deal), picks
  the argmax, and reports the **between-hypothesis margin**. This is
  the operation MYCIN actually performs and the engine previously
  lacked — nothing ranked competing conclusions or measured the gap.
- `DifferentialDecision` — `Determinate { leader, margin }` when the
  leader out-ranks the runner-up *even under the worst-case resolution
  of every open uncertainty* (leader's VOI band pushed down, runner-up's
  up); `Kickback { leader, runner_up, recommended_resolutions }` when
  the bands cross (an unresolved finding — or an exact tie — could flip
  the ranking). This is the cross-hypothesis analogue of
  `LRAggregateResult::suggest_kickback`, which only bounded a single
  hypothesis. Decision = argmax + sensitivity (ADJ65), deterministic and
  CPU-only — no softmax, no temperature.
- `RankedHypothesis` carries each hypothesis's full `LRAggregateResult`
  (proof DAG included) so the differential is auditable end to end, plus
  a `normalized_share` (posterior ÷ Σ posteriors) flagged as a
  display-only convenience that assumes the hypotheses are exhaustive and
  mutually exclusive (the LR model does not).
- Re-exported `differential`, `Differential`, `DifferentialDecision`,
  `RankedHypothesis` from the crate root.

## [0.6.0] - 2026-06-02

### Added

- `counterfactual(query, kb, &[Term])` — clones the KB, adds the
  given Facts as Certain, and reruns `lr_aggregate`. Lets the
  caller answer "what would the posterior be if X were true?"
  without disturbing the original KB. Cloning the whole KB makes
  the contract obvious; cost is linear and small.
- `LRAggregateResult::suggest_kickback(decision_threshold)` —
  computes a worst-case / best-case posterior band by reducing
  each active uncertainty marker to its min/max contribution,
  summing the shifts independently across markers, and applying
  to the current `posterior_logit`. Returns `Some(KickbackReport)`
  iff the band straddles `decision_threshold`. Includes
  `recommended_resolutions` sorted by individual VOI.
- `source_disagreements(kb, conclusion)` /
  `source_disagreements_with_threshold(kb, conclusion, min_spread)`
  — scans contributions on `conclusion`, groups by `evidence_term`,
  flags groups where the `logit_delta`s have spread > threshold.
  Per-source records include the clause id and provenance so the
  audit reader can render "AHA 2021 says LR=2.5; ESC 2023 says
  LR=4.0; sources disagree by 0.47 logits."
- New types: `KickbackReport`, `SourceDisagreementReport`,
  `SourceLogitDelta`.
- `KnowledgeBase: Clone`.
- 7 new tests covering counterfactual upward shift, KB
  non-mutation invariant, kickback firing inside the band, no
  kickback outside the band, source-disagreement detection on two
  conflicting sources, no-disagreement when only one source, and
  no-disagreement when sources agree.

Total tests: 69 (was 62 in 0.5.0).

### ADJ46 awkwardness items dissolved by 0.6.0

- **A7** (no kickback search variant) — addressed via
  `suggest_kickback` method on the result rather than a separate
  search mode. Lower-friction API and the same diagnostic power.
- **A8** (counterfactuals require KB clone + rerun) — `counterfactual`
  function does the clone + rerun once, atomically; caller's KB
  is invariant.
- **A9** (source-disagreement aggregation) — detector +
  per-source records surface conflicting LRs from the rulebook.

### Status of the original 10 ADJ46 awkwardness items

| Item | Status |
|---|---|
| A1 (LR magnitudes) | ✅ 0.3.0 |
| A2 (provenance) | ✅ 0.4.0 |
| A3 (prior) | ✅ 0.3.0 |
| A4 (joint contributions syntax) | ✅ 0.1.0 (adj-lang) |
| A5 (uncertainty markers) | ✅ 0.5.0 |
| A6 (WMC vs LR) | ✅ 0.3.0 |
| A7 (kickback) | ✅ 0.6.0 |
| A8 (counterfactuals) | ✅ 0.6.0 |
| A9 (source disagreement) | ✅ 0.6.0 |
| A10 (surface syntax) | ✅ 0.1.0 (adj-lang) |

All ten items dissolved as of logic-engine 0.6.0 +
adj-lang 0.2.0.

## [0.5.0] - 2026-06-02

### Added

- `UncertaintyMarker` clause type + `UncertaintyMarkerId`. Attached
  to a conclusion with a `domain: Vec<Term>` of candidate evidence
  terms. Represents "the IR pipeline knows the conclusion is the
  target of an LR query, and knows the patient (or source) did not
  specify one of these candidate values."
- `UncertaintyReport` — the user-facing VOI summary the engine
  emits when a marker's domain is entirely unobserved. Contains the
  domain, the log-odds delta each value would have contributed if
  observed, and a v0.1 VOI proxy (`voi_logit_range` = max − min of
  the deltas). The framework's user-facing layer can rank these to
  produce "if you can determine X, the posterior could swing by up
  to Y" guidance.
- `LRAggregateResult.uncertainties: Vec<UncertaintyReport>` +
  `SearchResult::LRAggregateResult { ..., uncertainties }`.
- `KnowledgeBase::add_uncertainty_marker` /
  `uncertainty_markers_for`. Markers do not promote a query to
  LR-aggregation — they're only meaningful relative to contribution
  clauses already on the conclusion.
- 3 new integration tests in `tests/test_lr_aggregation.rs`:
  uncertainty report with no observation shows full domain + VOI,
  one-domain-observation suppresses the report, marker over a
  domain with no matching contributions has zero VOI but still
  appears in the report.

Total tests: 62 (was 59 in 0.4.0).

### ADJ46 awkwardness items dissolved by 0.5.0

- **A5** — uncertainty markers at the engine layer.
  `add_uncertainty_marker` + `UncertaintyReport` give the IR
  pipeline a way to losslessly hand off "the patient said nothing
  about X over this domain" to the executor, and give the audit
  reader a concrete VOI signal to act on.

### Scope notes

- VOI is the v0.1 proxy (max − min over candidate log-odds deltas)
  — not the formal Bayesian decision-theoretic VOI. A richer
  treatment that combines the candidate deltas with the prior over
  the domain (and with the user's decision threshold, if any) is a
  follow-up.
- Still pending: A7 (kickback variant), A8 (counterfactuals), A9
  (multi-source aggregation), and the surface-layer half of A5
  (the `uncertain { ... } for ...` keyword — that ships in
  `adj-lang` 0.2.0 simultaneously).

## [0.4.0] - 2026-06-02

### Added

- `provenance` module: `Provenance { source, locator, trust_tier }`
  + `TrustTier { Consensus, Authoritative, Empirical, Inferred,
  Unattributed }`. Designed so the common case is a one-liner —
  `Provenance::cited("AHA 2021 §3.2")` — while still carrying enough
  structure that an audit reader can sort or filter across clauses
  by trust tier.
- `PriorClause`, `ContributionClause`, `JointContributionClause`
  each grow a `provenance: Provenance` field plus a
  builder-style `.with_provenance(...)` method. Default is
  `Provenance::unattributed()` so existing pre-ADJ47-B code
  continues to construct clauses without any source-of-truth
  ambiguity.
- 5 new inline unit tests in `provenance.rs` covering trust-tier
  ordering, locator builder-style threading, and the default.
- 2 new integration tests in `tests/test_lr_aggregation.rs`:
  `provenance_is_recoverable_from_kb_after_aggregation` (the
  contract: clauses carry citations and the audit reader recovers
  them via the clause id from the proof DAG, no side-table) and
  `unattributed_provenance_is_the_default` (legacy compatibility).

Total tests: 59 (was 52 in 0.3.0).

### ADJ46 awkwardness items dissolved by 0.4.0

- **A2** (provenance is not a clause field) — fully addressed.
  Clauses now carry citations; the proof DAG references them by
  clause id; no side-table required.

### Scope notes

What 0.4.0 still does NOT ship: A4 (joint as syntactically
distinct from atomic in the *surface* syntax — semantically the
engine already distinguishes them), A5 (uncertainty markers), A7
(kickback search variant), A8 (counterfactuals), A9 (source-
disagreement aggregation, though `Provenance` is the prerequisite
data structure), A10 (surface syntax) — all language-layer.

## [0.3.0] - 2026-06-02

### Added

- `lr_aggregate` module: full implementation of
  [`LP19e`](../../../specs/LP19e-likelihood-ratio-aggregation.md)
  likelihood-ratio Bayesian aggregation. Three new clause types
  (`PriorClause`, `ContributionClause`, `JointContributionClause`)
  plus three new id types, an `lr_aggregate(query, kb)` function,
  numerically stable `sigmoid` / `logit` helpers, an
  `LRAggregateResult` carrying the proof DAG and posterior, and an
  `LrAggregateWarning` enum surfacing the LP19e §"Edge cases"
  (no prior declared, no contributions active, degenerate LR=1.0
  contribution).
- `SearchMode::LRAggregate` variant + `SearchResult::LRAggregateResult`
  variant. `AutoDetect` now routes to `LRAggregate` first whenever
  `kb.participates_in_lr_aggregation(query)` is true, then falls
  back to the LP19 short-circuit between `FindFirst` and
  `EnumerateAll`.
- `KnowledgeBase` extensions: `add_prior`, `add_contribution`,
  `add_joint_contribution`, `prior_for`, `contributions_for`,
  `joint_contributions_for`, `participates_in_lr_aggregation`,
  `observed_evidence`. The new storage is flat `Vec`s rather than
  `HashMap<Term, _>` because `Term` does not implement
  `Hash + Eq`; linear scan is fine at current scale and switching
  to an indexed map later is purely additive.
- `DerivationOrigin` grows three additive variants: `FromPrior`,
  `FromContribution`, `FromJointContribution`. Each carries the
  log-odds delta inline so an audit reader can reconstruct running
  log-odds from the proof's `steps` without consulting the KB.
- `Proof` grows two additive fields: `posterior_logit:
  Option<f64>` and `posterior_probability: Option<f64>`. `Some(_)`
  on LR-aggregation proofs, `None` on SLD / WMC proofs.
- 7 integration tests in `tests/test_lr_aggregation.rs` covering
  the ADJ36 ACS chest-pain scenario end-to-end (reproduces 28.1%
  posterior), `AutoDetect` routing, missing-prior warning, joint
  contributions, evidence Fact id threading into the proof DAG,
  compound-term equality on the linear-scan lookup, and conflicting
  priors rejection.
- 9 inline unit tests in `lr_aggregate.rs` covering numeric
  stability, round-trip through `logit`/`sigmoid`, constructor
  panics on out-of-range inputs, the prior-only case, single and
  joint contributions, unobserved evidence skipped, and
  `KbError::ConflictingPriors`.

Total tests: 52 (was 36 in 0.2.0).

### Scope notes

This slice dissolves the engine-layer half of ADJ46's awkwardness
catalogue at items A1 (LR magnitudes), A3 (Bayesian prior), A6 (WMC
discarded; we now compute the right posterior), and starts on A2
(provenance — id types are now distinct so the audit trail can name
the clause kind, though source-citation fields on clauses themselves
are still ADJ47 follow-up work).

What 0.3.0 does NOT yet ship: counterfactual queries (A8),
source-disagreement aggregation (A9), uncertainty markers (A5),
kickback variant (A7), or a surface syntax (A10) — all are language-
layer and live in ADJ47.

## [0.2.0] - 2026-05-11

### Added

- `proof_dag` module: `ProofDAG`, `Proof`, `ProofStep`, `DerivationOrigin`
  — the engine's return type when enumeration is active. Each `Proof`
  records its final substitution, an ordered list of derivation steps,
  and de-duplicated `via_facts` / `via_rules` lists that name every
  probabilistic clause the proof depends on.
- `enumerate` module: `enumerate_all(query, kb)` — exhaustive SLD that
  collects every successful derivation rather than stopping at the
  first. Uses the same fresh-variable renaming as `find_first` so that
  multiple clause instantiations don't share variable identity.
  Negation-as-failure is the well-founded reading per LP19.
- `wmc` module: `weighted_model_count(dag, kb)` — naïve enumeration
  over `2^n` worlds, where `n` is the count of distinct probabilistic
  clauses across all proofs. Certain clauses are automatically true
  and do not contribute degrees of freedom. The shared-fact case is
  handled correctly because WMC counts worlds, not paths.
- `SearchResult` enum with `FindFirstResult` and `EnumerateAllResult`
  variants, plus a top-level `search(query, kb, mode)` function. In
  `AutoDetect` mode the engine inspects `kb.is_all_certain()` and
  selects `FindFirst` when every clause is `Certain` — the LP19
  short-circuit theorem made executable.
- `KnowledgeBase::find_fact_by_id` and `find_rule_by_id` — linear-scan
  lookup used by the WMC backend to recover Bernoulli parameters from
  clause ids. Sufficient at current scale; an indexed alternative may
  arrive in a later slice.
- 11 new tests (4 enumerate, 7 wmc, 4 integration) including the
  canonical `P(path(a,c)) = 0.86` graph reachability and the
  shared-fact case that fails under naïve inclusion-exclusion (correctly
  returns 0.5 here, would be 0.75 under the wrong algorithm). Total:
  30 tests.

### Scope notes

This slice completes the probabilistic core specified in
[`LP19`](../../../specs/LP19-probabilistic-logic-core.md) for the naïve
inference path. `d-DNNF` / `SDD` compilation (LP19a), rational
arithmetic (LP19b), conditional probability with evidence (LP19c), and
approximate inference (LP19d) remain as planned follow-ups.

## [0.1.0] - 2026-05-11

### Added

- `Probability` enum with two variants: `Certain` (semantic 1.0, recognized
  structurally for the LP19 short-circuit) and `Value(f64)` for genuine
  probabilities in `[0, 1]`.
- `Fact { id, term, probability }` carrying a stable `FactId` and a
  probability that defaults to `Certain`.
- `Rule { id, head, body, probability }` with `BodyLiteral::{Pos, Neg}`
  body literals (positive goals and negation-as-failure).
- `KnowledgeBase` — an indexed collection of Facts and Rules. Looks up
  by head functor/arity for fast clause selection during search.
- `SearchMode` enum: `FindFirst`, `EnumerateAll`, `AutoDetect`.
- `is_all_certain()` on `KnowledgeBase` — the precondition for the
  LP19 short-circuit. The implementation walks every Fact and Rule once
  and is `O(|KB|)`.
- `find_first(query, kb)` — deterministic SLD-style resolution over the
  KB. Returns the first successful `Substitution` or `None`. Uses the
  unification from `logic-core` and the KB's clause index for clause
  selection.
- 14 tests covering: deterministic facts, rules with bodies, multiple
  clauses with backtracking, the all-Certain short-circuit detection,
  rejection of anonymous probabilistic clauses (well-formedness), and a
  small "family relations" worked example used by the LP layer's
  educational specs.

### Scope

This is the first slice of [`LP19`](../../../specs/LP19-probabilistic-logic-core.md).
Subsequent slices will add:

- Proof DAG construction (return all successful derivations, not just
  the first).
- `EnumerateAll` and `AutoDetect` mode implementations.
- Naïve weighted-model-counting backend over the proof DAG's induced
  Boolean formula.
- d-DNNF / SDD compilation (LP19a).

The current slice deliberately limits itself to **deterministic
find-first search** so that the foundation can be reviewed before the
probabilistic backend is added.

### Notes

The Python reference at `code/packages/python/logic-engine` remains the
canonical interpretation. The Rust crate currently is a strict subset
of the Python API surface; subsequent PRs expand to parity.
