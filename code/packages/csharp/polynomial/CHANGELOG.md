# Changelog — CodingAdventures.Polynomial (C#)

All notable changes to this package are documented here.

## [0.1.0] — 2026-04-24

### Added

- Initial release implementing the MA00 polynomial specification.
- `Polynomial.Normalize(double[])` — strip trailing zero coefficients.
- `Polynomial.Degree(double[])` — highest non-zero index, or −1 for the zero polynomial.
- `Polynomial.Zero()` — return the additive identity `[]`.
- `Polynomial.One()` — return the multiplicative identity `[1]`.
- `Polynomial.Add(double[], double[])` — term-by-term addition with implicit zero padding.
- `Polynomial.Subtract(double[], double[])` — term-by-term subtraction.
- `Polynomial.Multiply(double[], double[])` — polynomial convolution.
- `Polynomial.DivMod(double[], double[])` — long division returning `(quotient, remainder)` tuple.
- `Polynomial.Divide(double[], double[])` — quotient only.
- `Polynomial.Mod(double[], double[])` — remainder only.
- `Polynomial.Evaluate(double[], double)` — Horner's method evaluation.
- `Polynomial.Gcd(double[], double[])` — Euclidean algorithm GCD.
- `Polynomial.Format(double[])` — human-readable polynomial string (debug helper).
- Full xUnit test suite with >90% line coverage.
- `BUILD` and `BUILD_windows` scripts for CI integration.
