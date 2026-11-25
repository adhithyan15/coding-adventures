## [0.4.1] - 2026-07-11

### Added

- **`BigRational::to_f64()`** and **`BigDecimal::to_f64()`** — the **labeled lossy `f64`
  export** the ADJ engine needs for NUM-5 (arbitrary precision is the default; `f64` is a
  boundary export a consumer explicitly asks for). `BigRational::to_f64` divides the exact
  `num / den` as a `BigDouble` (numerator and denominator entering it exactly, capped at
  `MAX_PRECISION`) at `f64` width plus guard bits, so the result is the correctly-rounded
  nearest `f64` for any rational of practical size and saturates cleanly for extreme
  magnitudes (`10^400 → ∞`, `10^-400 → 0`); `BigDecimal::to_f64` routes through the value's
  plain-decimal string and Rust's correctly-rounded float parser. Tested: exact/dyadic
  round-trips, a 5,000-case differential of `to_f64` against hardware `n as f64 / d as f64`,
  and extreme-magnitude saturation without panic.
