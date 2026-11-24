## [0.8.0] - 2026-08-02

### Added

- **`BigDouble::from_rational(r: &BigRational, prec: u32, mode: RoundingMode) -> BigDouble`** (NUM-7a)
  — the promotion primitive from the exact `BigRational` world into the approximate `BigDouble`
  world: the counterpart to `BigRational::to_f64`, generalized from a fixed 53-bit `f64` target to a
  caller-requested precision. Both parts enter exactly (each at its own bit length, capped at
  `MAX_PRECISION`), and the quotient is taken at `prec` bits so the result is the correctly rounded
  `BigDouble` for any rational of practical size. Introduced so the logic engine can wire a
  per-`KnowledgeBase`-configurable `Real`/`BigDouble` audit companion (ADJ-NUMERIC-SUBSTRATE §8).
