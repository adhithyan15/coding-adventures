# Changelog

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-04-05

### Added

- `ReedSolomon.buildGenerator(_:)` — Build the RS generator polynomial for a given
  number of check bytes. Returns a little-endian coefficient array of length `nCheck+1`.
  Cross-language test vector: `buildGenerator(2)` = `[8, 6, 1]`.
- `ReedSolomon.encode(_:nCheck:)` — Systematic encoding: produces a codeword of length
  `message.count + nCheck` where the first `k` bytes are the original message and the
  last `nCheck` bytes are error-correction check bytes. Throws `InvalidInput` if
  `nCheck` is 0 or odd, or if total length exceeds 255.
- `ReedSolomon.syndromes(_:nCheck:)` — Compute `nCheck` syndrome values. Returns all
  zeros for a valid codeword; non-zero syndromes indicate corruption.
- `ReedSolomon.errorLocator(_:)` — Berlekamp-Massey algorithm: finds the shortest LFSR
  (error locator polynomial `Λ(x)`) that generates the syndrome sequence. Returns a
  little-endian array with `Λ[0] = 1`.
- `ReedSolomon.decode(_:nCheck:)` — Five-step decode pipeline:
  1. Compute syndromes
  2. Berlekamp-Massey → `Λ(x)` and error count
  3. Chien search → error positions
  4. Forney algorithm → error magnitudes
  5. Apply corrections

  Corrects up to `t = nCheck/2` byte errors. Throws `TooManyErrors` if the codeword
  is too damaged, or `InvalidInput` for bad parameters.
- `ReedSolomon.TooManyErrors` — Error type thrown when more than `t` errors are present.
- `ReedSolomon.InvalidInput` — Error type thrown for bad parameters, carrying a
  `.reason` string.
- Private helper functions:
  - `polyEvalBE` — Horner evaluation of big-endian GF(256) polynomials.
  - `polyEvalLE` — Horner evaluation of little-endian GF(256) polynomials.
  - `polyMulLE` — Schoolbook convolution in GF(256) (little-endian).
  - `polyAddLE` — Coefficient-wise XOR for little-endian polynomials.
  - `polyScaleLE` — Scalar multiplication of little-endian polynomials.
  - `polyModBE` — Synthetic division remainder for big-endian polynomials.
- Comprehensive test suite with 43 test cases covering:
  - Cross-language test vector for `buildGenerator(2)`
  - Root property verification for all generators
  - Systematic encoding and zero-syndrome property
  - Error correction at all single positions
  - Two- and four-error correction
  - `TooManyErrors` and `InvalidInput` thrown correctly
  - Round-trip properties for all 256 single-byte messages
- Literate programming style with extensive inline documentation explaining
  RS theory, polynomial conventions, Berlekamp-Massey, Chien search, and
  the Forney algorithm.

### Design Notes

- All public API lives in `public enum ReedSolomon` (namespace enum, cannot be
  instantiated) to avoid name collisions and keep call sites clean.
- Polynomial convention is clearly documented: codewords are big-endian (BE),
  internal polynomial algebra is little-endian (LE). The transition points are
  explicitly marked in each function.
- GF(256) arithmetic is delegated entirely to the `GF256` package (MA01).
  This package adds only the polynomial and syndrome machinery on top.
- `polyModBE` reverses the LE generator to BE internally before division,
  matching the Python/TypeScript reference implementations.
- `buildGenerator` validation (nCheck > 0, nCheck % 2 == 0) is intentionally
  kept in `buildGenerator` so it is always enforced, even when called directly.
