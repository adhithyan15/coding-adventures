# Changelog

All notable changes to the C++ `activation-functions` package are documented
here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial header-only pure-ISO C++17 port of the Rust `activation-functions`
  crate (namespace `ca::activation_functions`).
- Six activations plus their derivatives: `linear`, `sigmoid`, `relu`,
  `leaky_relu`, `tanh`, `softplus` (and the matching `*_derivative`), plus the
  `LEAKY_RELU_SLOPE` constant.
- libm-free transcendentals in `detail`: `e^x` (Cody-Waite range reduction +
  Taylor series + exact `2^k`), `ln(1+x)` (`2·atanh` form), and `tanh` (stable
  ratio) — matching `std::exp` / `std::tanh` / `std::log1p` to within ~1e-12.
- Numerically stable, total definitions: sigmoid saturates outside ±709,
  softplus uses `ln(1 + e^-|x|) + max(x, 0)`. `e^x` guards both overflow and
  large-negative underflow (the latter also keeps huge softplus arguments out
  of the internal `double`->`int` range reduction, avoiding UB), and propagates
  NaN.
- 45 checks against the Rust crate's reference values (1e-12 tolerance) plus
  extreme/non-finite inputs, run under every available C++ compiler via the
  shared `iso-harness`.
