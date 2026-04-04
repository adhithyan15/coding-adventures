# Changelog — coding-adventures-reed-solomon

All notable changes are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-04-04

### Added

- **`encode(message, n_check)`** — Systematic RS encoding over GF(256). Message
  bytes preserved at the start of the output; `n_check` check bytes appended.
- **`decode(received, n_check)`** — 5-step syndrome decoding pipeline:
  syndromes → Berlekamp-Massey → Chien search → Forney → byte correction.
  Corrects up to `t = n_check // 2` byte errors.
- **`syndromes(received, n_check)`** — Compute `n_check` syndrome values.
  All-zero means no errors; any non-zero signals corruption.
- **`build_generator(n_check)`** — Monic generator polynomial `g(x) = ∏(x+αⁱ)`
  in little-endian form.
- **`error_locator(syndromes)`** — BM error locator polynomial for external use.
- **`TooManyErrorsError`** — Raised when correction capacity is exceeded.
- **`InvalidInputError`** — Raised for bad `n_check` or oversized codewords.
- Full test suite: round-trips, error correction at every byte position,
  capacity limits, test vectors, edge cases. Cross-validated against Rust.

### Dependencies

- `coding-adventures-gf256 >= 0.1.0` — all field arithmetic.

### Notes

- Polynomial conventions match the Rust and TypeScript references: big-endian
  codeword bytes, little-endian internal polynomials (Λ, Ω, generator).
- Locator inverse formula: `X_p⁻¹ = α^{(p+256-n) mod 255}` — consistent
  across Chien search and Forney, matching Rust/TypeScript.
