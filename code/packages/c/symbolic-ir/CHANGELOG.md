# Changelog

All notable changes to the C `symbolic-ir` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.2.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `symbolic-ir` crate (version tracks the
  Rust crate at 0.2.0).
- `SirNode` tagged union with six variants (Symbol, Integer, Rational, Float,
  Str, Apply) and malloc-owned constructors `sir_sym` / `sir_int` /
  `sir_rational` / `sir_flt` / `sir_str` / `sir_apply` (the last consumes its
  head + args), with recursive `sir_free`.
- `sir_rational`: GCD reduction, sign moved to the numerator, collapse to
  Integer when the denominator reduces to 1; `SIR_ERR_ZERO_DENOM` in place of
  the Rust panic. Uses two's-complement magnitude arithmetic so `INT64_MIN`
  inputs carry no signed-overflow UB.
- Structural `sir_equals` (floats by bit pattern), a consistent FNV-1a
  `sir_hash`, and a recursive `sir_to_string` with shortest round-tripping float
  formatting.
- The standard head-name vocabulary as `SIR_ADD` … `SIR_RULE` macros.
- 63 checks against the Rust crate's reference behavior, run under every
  available C compiler via the shared `iso-harness`.
