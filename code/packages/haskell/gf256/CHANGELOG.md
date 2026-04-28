# Changelog

All notable changes to the `gf256` Haskell package will be documented here.

## [0.1.0.0] - 2026-04-24

### Added

- Initial implementation of GF(256) field arithmetic (MA01).
- `GF256` type alias (`Int` in range [0, 255]).
- `primitivePoly = 0x11D`: the Reed-Solomon irreducible polynomial.
- `expTable` and `logTable`: precomputed O(1) log/antilog tables.
- `gfAdd`, `gfSub`: XOR-based addition and subtraction.
- `gfMul`: O(1) multiplication via log/antilog tables.
- `gfDiv`: O(1) division via log/antilog tables.
- `gfPow`: exponentiation using the cyclic group of order 255.
- `gfInv`: multiplicative inverse using `expTable[255 - logTable[a]]`.
- Full Hspec test suite with field axiom verification and cross-checks
  against spec test vectors from MA01-gf256.md.
