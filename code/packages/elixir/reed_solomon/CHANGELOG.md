# Changelog — ReedSolomon (Elixir)

All notable changes are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-04-04

### Added

- **`ReedSolomon.encode(message, n_check)`** — Systematic RS encoding over GF(256).
  Message bytes preserved at start; `n_check` check bytes appended.
- **`ReedSolomon.decode(received, n_check)`** — 5-step syndrome decoding pipeline:
  syndromes → Berlekamp-Massey → Chien search → Forney → byte correction.
  Corrects up to `t = n_check / 2` byte errors.
- **`ReedSolomon.syndromes(received, n_check)`** — Compute `n_check` syndrome
  values. All-zero means no errors; any non-zero signals corruption.
- **`ReedSolomon.build_generator(n_check)`** — Monic generator polynomial
  `g(x) = ∏(x+αⁱ)` in little-endian form.
- **`ReedSolomon.error_locator(syndromes)`** — BM error locator for external use.
- **`TooManyErrors`** — Raised when correction capacity is exceeded.
- **`InvalidInput`** — Raised for bad `n_check` or oversized codewords.
- 45 tests. Cross-validated against Rust, TypeScript, Python, Go, and Ruby.

### Dependencies

- `coding_adventures_gf256` (local: `../gf256`) — all field arithmetic.

### Notes

- Polynomial conventions match all reference implementations: big-endian codeword
  bytes, little-endian internal polynomials (Λ, Ω, generator).
- Locator inverse formula: `X_p⁻¹ = α^{(p+256-n) mod 255}` — consistent
  across Chien search and Forney.
- Elixir reserved words (`after`, `rescue`, etc.) avoided as variable names.
