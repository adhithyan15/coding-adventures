# LP19b — Rational Arithmetic for Exact-Precision Probability

## Overview

[`LP19`](LP19-probabilistic-logic-core.md) and
[`LP19a`](LP19a-d-dnnf-compilation.md) describe the engine and its
scaling path, both using `f64` (double-precision floating point) for
probability arithmetic. This sub-spec defines an **exact-precision**
mode in which probabilities are represented as **rationals** (ratios
of arbitrary-precision integers), and weighted model counting
evaluates exactly.

Exact mode matters in three situations:

1. **Rare-disease priors.** A prior of `1e-9` and a downstream
   product of `1e-9 × 0.95 × 0.80 × ...` lands in the underflow zone
   of `f64` after a few multiplications. The answer is mathematically
   well-defined but numerically zero in float.
2. **Regulatory or legal audit.** A clinical decision support tool
   asked to defend a probability of `0.137` must be able to reproduce
   *exactly* that probability, not "approximately". Rational
   arithmetic makes this trivial; float arithmetic exposes you to
   reproducibility questions across hardware (different SSE/AVX
   instruction ordering produces different last-bit results).
3. **Probabilities published as fractions.** Drug-interaction
   literature often gives likelihood ratios as `3/5` or `7/12`. The
   most faithful representation is the fraction itself, not its
   float-rounded decimal expansion.

The exact-mode engine produces a *rational number*, not a float,
which the caller can render however they want (as a fraction, a
decimal, or both). Round-trip exactness is guaranteed.

## Layer Position

```
   LP19  probabilistic logic core                ← f64 by default
        │
        ├── LP19a  d-DNNF compilation             (scales the formula side)
        │
        └── LP19b  rational arithmetic            ← this spec (scales the
                                                    *number* side)
        │
        ▼
   logic-engine v4 with optional Rational backend
```

LP19a and LP19b are independent and compose. A deployment can use
d-DNNF + f64 (the common case), naïve + Rational (small KB, exact),
d-DNNF + Rational (small KB, exact, fast), or naïve + f64 (the
default starting point).

## The Rational Type

Two implementations supported:

1. **Fixed-precision `Rational<i128>`** (default). A numerator/denominator
   pair, each an `i128`. Sufficient for ~38 decimal digits of precision
   on each side. Fast (essentially native arithmetic) and zero-allocation.
2. **Arbitrary-precision `Rational<BigInt>`** (opt-in). Backed by a big-
   integer crate (likely `num-bigint`'s `Rational64`/`BigRational`).
   Slower per operation but never overflows.

Both expose the same trait:

```text
trait ProbabilityWeight: Copy + Add + Mul + Sub + One + Zero {
    fn from_fraction(numerator: i64, denominator: i64) -> Self;
    fn from_f64(p: f64, max_denominator: i64) -> Self;
    fn as_f64(&self) -> f64;
    fn as_fraction(&self) -> (i128, i128);   // numerator, denominator
}
```

`f64` implements this trait as well, so the engine is generic over
the weight type and the same code paths serve both modes.

## Representation

A rational `p/q` is stored in **canonical form**: `gcd(p, q) = 1` and
`q > 0`. After every arithmetic operation the result is reduced to
canonical form. This keeps the numerator and denominator from
growing unnecessarily, but cannot prevent growth in the worst case
(see "Overflow Handling" below).

The default `from_f64` approximates a float by repeated `gcd` against
a target `max_denominator` (configurable; default `1_000_000_000`).
Users who know the exact fraction (e.g., `3/5`) should call
`from_fraction(3, 5)` directly.

## Arithmetic

Standard rational arithmetic. Sum, product, difference, all reduced
to canonical form after each operation:

```text
(a/b) + (c/d)  =  (ad + bc) / (bd)   then reduce
(a/b) × (c/d)  =  (ac) / (bd)        then reduce
(a/b) − (c/d)  =  (ad − bc) / (bd)   then reduce
```

Division is not exposed at the weight-type level (WMC never divides;
the conditional-probability formula `P(Q ∧ E) / P(E)` from
`LP19c` is the only division and it happens *after* both numerator
and denominator have been computed).

## Overflow Handling

The fixed-precision `Rational<i128>` is fast but bounded. After
enough operations the numerator or denominator may exceed `i128::MAX`.
The engine handles this in two layers:

1. **Detect.** Every arithmetic op checks for overflow. On detection
   the engine produces a `WmcError::WeightOverflow { context }` rather
   than a wrong answer.
2. **Promote.** When overflow occurs in a long-running computation,
   the engine optionally promotes the in-progress weight to
   `Rational<BigInt>` and continues. Promotion is a configuration
   knob; default is "promote on overflow" for high-stakes deployments
   and "error on overflow" for performance-critical ones.

## Performance Tradeoffs

Rational arithmetic is **substantially slower** than float:

- `i128` rationals: roughly 5–10× slower than `f64` for `+` and `×`.
  GCD reductions dominate the cost.
- `BigInt` rationals: 50–500× slower depending on numerator/denominator
  size. Heap allocations on every op.

For typical adjudication-scale KBs (≤100 probabilistic clauses,
single-digit-thousand proof DAG nodes), `i128` rationals add
seconds, not minutes. Use `BigInt` only when you know the precision
matters and the deployment can tolerate the cost.

## API Sketch

```rust
// Logic-engine extension:

pub trait ProbabilityWeight: Copy {
    fn one() -> Self;
    fn zero() -> Self;
    fn add(self, other: Self) -> Self;
    fn mul(self, other: Self) -> Self;
    fn sub(self, other: Self) -> Self;
    fn from_fraction(num: i64, den: i64) -> Self;
    fn as_f64(&self) -> f64;
}

impl ProbabilityWeight for f64 { /* ... */ }
impl ProbabilityWeight for Rational128 { /* ... */ }
impl ProbabilityWeight for RationalBig { /* ... */ }

pub fn weighted_model_count_generic<W: ProbabilityWeight>(
    dag: &ProofDAG,
    kb: &KnowledgeBase,
) -> Result<W, WmcError>;
```

The existing `weighted_model_count` becomes
`weighted_model_count_generic::<f64>` for backwards compatibility.

## Mode Selection

A search-mode configuration field selects the weight type:

```text
WmcMode :=
    Float64                                    -- default
  | RationalI128 { promote_on_overflow: bool }
  | RationalBigInt                              -- always arbitrary precision
```

The mode is part of the audit-trail's configuration and is reproducible
across replay (`ADJ07` / `ADJ08`).

## Worked Example: Rare-Disease Prior

```text
0.0000001 :: rare_disease.       % 1 in 10 million
0.95 :: positive_test :- rare_disease.
0.0001 :: positive_test :- \+ rare_disease.

?- evidence(positive_test, true), query(rare_disease).
```

Under `f64`, the conditional posterior is computable but loses
precision in the deep multiplication. Under `Rational<i128>`:

```text
P(rare_disease ∧ positive_test) = (1/10_000_000) × (95/100)
                                = 95 / 1_000_000_000_000
P(positive_test) = (1/10_000_000) × (95/100)
                 + (9_999_999/10_000_000) × (1/10_000)
                 = 95/1_000_000_000_000 + 9_999_999/100_000_000_000_000
                 = (after common denominator and reduction) ...

P(rare_disease | positive_test) = numerator / denominator
                                ≈ 0.000949 (exact rational)
```

The exact rational is preserved and can be rendered as either a
decimal or a fraction. The clinical decision support tool can
defend the answer by showing its fractional form.

## Comparison with Other Implementations

ProbLog 2 (KU Leuven) defaults to floating-point WMC and offers a
rational mode via Python's `Fraction` type for evaluation. The
Rust engine adopts the same conceptual mode split but pushes
performance via fixed-precision rationals as the first stop.

LIBPME (a research probabilistic ML engine) uses arbitrary-precision
rationals throughout and pays a large constant factor; the LP19b
default favors fixed-precision rationals because most KBs do not
require arbitrary precision.

## Open Questions

1. **Mixed-mode evaluation**: can the engine evaluate part of the
   formula in float and part in rational? In principle yes, but the
   semantics of conversion are subtle (when do you promote? when do
   you demote?). Out of scope for the first version.
2. **Algebraic identities**: a rational engine could evaluate
   `(1 - p)` as `(q - p_num) / q` directly instead of `1 - (p_num /
   p_den) = (p_den - p_num) / p_den` and skipping a reduction step.
   Optimization opportunity, not a correctness issue.
3. **Rounding output**: should the engine truncate or round when
   converting the final rational back to a float for the caller?
   Round-half-to-even is the obvious default; configuration knob
   for callers who care.
4. **Rational d-DNNF**: combining `LP19a`'s d-DNNF with `LP19b`'s
   Rational weights is straightforward (the d-DNNF nodes are
   generic over the weight type). Test coverage is the only work.

## Limitations

1. **Inputs as floats lose precision before they reach the engine.**
   A clinician who enters "0.7" gets `0.7000...01`-ish in float
   regardless of engine mode. Exact mode protects you against
   *computation* losses, not *input* losses.
2. **Display rounding still happens.** "P = 137/1000" displayed as
   "0.137" rounds; the engine itself preserves the rational, but
   the human-facing display chooses a representation.
3. **Performance**. Rationals are slower than floats. The mode is
   opt-in for that reason; deployments where every millisecond
   matters (real-time triage) should stick with `Float64`.

## Status

Draft. Implementation is mostly mechanical given LP19's existing
WMC backend. The trait-based weight type sketch above is sufficient
to begin implementation; the engine's generic over `W: ProbabilityWeight`
will be the only invasive change to existing code.
