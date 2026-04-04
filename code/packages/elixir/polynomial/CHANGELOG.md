# Changelog — coding_adventures_polynomial (Elixir)

All notable changes to this package will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-03

### Added
- `CodingAdventures.Polynomial` module with full polynomial arithmetic
- `normalize/1` — strip trailing near-zero coefficients (threshold 1.0e-10)
- `degree/1` — return degree of polynomial (0 for zero polynomial)
- `zero/0` — additive identity `[0.0]`
- `one/0` — multiplicative identity `[1.0]`
- `add/2` — term-by-term polynomial addition
- `subtract/2` — term-by-term polynomial subtraction
- `multiply/2` — polynomial convolution
- `divmod_poly/2` — polynomial long division returning `{quotient, remainder}`
- `divide/2` — quotient only
- `modulo/2` — remainder only
- `evaluate/2` — Horner's method evaluation at a point
- `gcd/2` — Euclidean GCD returning monic result
- 45+ ExUnit tests covering all functions and edge cases
- Knuth-style literate programming comments throughout
