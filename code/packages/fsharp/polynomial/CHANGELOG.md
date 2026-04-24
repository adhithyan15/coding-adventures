# Changelog — CodingAdventures.Polynomial (F#)

All notable changes to this package are documented here.

## [0.1.0] — 2026-04-24

### Added

- Initial release implementing the MA00 polynomial specification.
- `Polynomial.normalize : Polynomial -> Polynomial` — strip trailing zero coefficients.
- `Polynomial.degree : Polynomial -> int` — highest non-zero index, or −1 for the zero polynomial.
- `Polynomial.zero : unit -> Polynomial` — return the additive identity `[||]`.
- `Polynomial.one : unit -> Polynomial` — return the multiplicative identity `[|1|]`.
- `Polynomial.add : Polynomial -> Polynomial -> Polynomial` — term-by-term addition.
- `Polynomial.subtract : Polynomial -> Polynomial -> Polynomial` — term-by-term subtraction.
- `Polynomial.multiply : Polynomial -> Polynomial -> Polynomial` — polynomial convolution.
- `Polynomial.divmod : Polynomial -> Polynomial -> Polynomial * Polynomial` — long division returning `(quotient, remainder)`.
- `Polynomial.divide : Polynomial -> Polynomial -> Polynomial` — quotient only.
- `Polynomial.pmod : Polynomial -> Polynomial -> Polynomial` — remainder only (named `pmod` to avoid shadowing `mod`).
- `Polynomial.evaluate : Polynomial -> float -> float` — Horner's method evaluation using `Array.foldBack`.
- `Polynomial.gcd : Polynomial -> Polynomial -> Polynomial` — Euclidean algorithm GCD.
- Full xUnit test suite with >90% line coverage.
- `BUILD` and `BUILD_windows` scripts for CI integration.
