# Changelog — reedsolomon (Go)

All notable changes are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-04-04

### Added

- **`Encode(message, nCheck)`** — Systematic RS encoding over GF(256). Message
  bytes preserved at the start of the output; `nCheck` check bytes appended.
- **`Decode(received, nCheck)`** — 5-step syndrome decoding pipeline:
  syndromes → Berlekamp-Massey → Chien search → Forney → byte correction.
  Corrects up to `t = nCheck/2` byte errors.
- **`Syndromes(received, nCheck)`** — Compute `nCheck` syndrome values.
  All-zero means no errors; any non-zero signals corruption.
- **`BuildGenerator(nCheck)`** — Monic generator polynomial `g(x) = ∏(x+αⁱ)`
  in little-endian form.
- **`ErrorLocator(synds)`** — BM error locator polynomial for external use.
- **`ErrTooManyErrors`** — Sentinel error for exceeded correction capacity.
- **`InvalidInputError`** — Typed error for bad `nCheck` or oversized codewords.
- 41 tests, 95.5% coverage. Cross-validated against Rust, TypeScript, and Python.

### Dependencies

- `github.com/adhithyan15/coding-adventures/code/packages/go/gf256` — all field arithmetic.

### Notes

- Polynomial conventions match the Rust/TypeScript/Python references: big-endian
  codeword bytes, little-endian internal polynomials (Λ, Ω, generator).
- Locator inverse formula: `X_p⁻¹ = α^{(p+256-n) mod 255}` — consistent
  across Chien search and Forney.
- Go idiom: no panics for recoverable errors; use typed error return values.
