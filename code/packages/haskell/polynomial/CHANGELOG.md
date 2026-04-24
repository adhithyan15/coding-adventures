# Changelog

All notable changes to the `polynomial` Haskell package will be documented here.

## [0.1.0.0] - 2026-04-24

### Added

- Initial implementation of polynomial arithmetic over GF(256) (MA00).
- `Poly` newtype wrapping a `[GF256]` coefficient list (little-endian).
- `polyNormalize`: strip trailing zero coefficients.
- `polyDegree`: highest non-zero index, −1 for the zero polynomial.
- `polyZero` / `polyOne`: additive and multiplicative identities.
- `polyAdd` / `polySub`: term-by-term XOR over GF(256).
- `polyScale`: scalar multiplication over GF(256).
- `polyMul`: polynomial convolution over GF(256).
- `polyDivMod`: polynomial long division over GF(256), returns (quotient, remainder).
- `polyDiv` / `polyMod`: quotient/remainder projections.
- `polyEval`: Horner's method evaluation at a GF(256) point.
- `polyGcd`: Euclidean algorithm GCD over GF(256).
- Full Hspec test suite verifying algebraic laws and Reed-Solomon use cases.
