# Changelog — @coding-adventures/reed-solomon

All notable changes are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-04-04

### Added

- **`encode(message, nCheck)`** — Systematic RS encoding over GF(256). Message
  bytes preserved at the start of the output; `nCheck` check bytes appended.
- **`decode(received, nCheck)`** — 5-step syndrome decoding pipeline:
  syndromes → Berlekamp-Massey → Chien search → Forney → byte correction.
  Corrects up to `t = nCheck / 2` byte errors.
- **`syndromes(received, nCheck)`** — Compute `nCheck` syndrome values.
  All-zero means no errors; any non-zero signals corruption.
- **`buildGenerator(nCheck)`** — Monic generator polynomial `g(x) = ∏(x+αⁱ)`
  in little-endian form.
- **`errorLocator(syndromes)`** — BM error locator polynomial for external use.
- **`TooManyErrorsError`** — Thrown when correction capacity is exceeded.
- **`InvalidInputError`** — Thrown for bad `nCheck` or oversized codewords.
- Full test suite: round-trips, error correction at every byte position,
  capacity limits, test vectors, edge cases. All cross-validated against Rust.

### Dependencies

- `@coding-adventures/gf256` (file:../gf256) — all field arithmetic.

### Notes

- Polynomial conventions match the Rust reference: big-endian codeword bytes,
  little-endian internal polynomials (Λ, Ω, generator).
- Locator inverse formula: `X_p⁻¹ = α^{(p+256-n) mod 255}` — consistent
  across Chien search and Forney.
