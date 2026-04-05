# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-05

### Added

- Initial implementation of `CodingAdventures::ReedSolomon`
- Public API: `encode`, `decode`, `syndromes`, `build_generator`, `error_locator`
- `build_generator($n_check)` — constructs monic generator polynomial g(x) as
  a little-endian array; cross-language test vector `build_generator(2) = [8, 6, 1]`
- `encode(\@message, $n_check)` — systematic encoding appending n_check check
  bytes computed as the remainder of M(x)·x^{n_check} divided by g(x)
- `syndromes(\@received, $n_check)` — evaluates received codeword at α^1..α^{n_check}
  via big-endian Horner's method
- `error_locator(\@syndromes)` — public wrapper for Berlekamp-Massey algorithm
- Berlekamp-Massey algorithm (`_berlekamp_massey`) — finds shortest LFSR / error
  locator polynomial Λ(x) from the syndrome sequence
- Chien search (`_chien_search`) — evaluates Λ at all X_p⁻¹ = α^{(p+256-n) mod 255}
  to find error positions
- Forney algorithm (`_forney`) — computes error magnitudes using formal derivative
  Λ'(x) in characteristic 2 (even-degree terms vanish) and error evaluator Ω(x)
- Five-step decode pipeline with early exit on all-zero syndromes
- Input validation with `InvalidInput` die messages for bad n_check or oversized
  codewords; `TooManyErrors` die messages when error count exceeds capacity t
- Knuth-style literate comments: truth tables, polynomial notation, algorithm
  sketches, cross-language conventions, and worked examples
- Test suite with 40+ subtests using Test2::V0, covering:
  - build_generator correctness, length, and monic property
  - encode length, systematic property, syndrome-zero invariant, invalid inputs
  - syndromes: valid=zero, corrupted≠zero
  - decode: no errors, single error, 2 errors (n_check=4), 4 errors (n_check=8),
    errors in check byte region, all-zeros/all-0xFF messages, large messages,
    TooManyErrors, InvalidInput
  - error_locator: all-zero syndromes → [1], Λ[0]=1, degree check
  - round-trip property: exhaustive single-error correction for all positions
