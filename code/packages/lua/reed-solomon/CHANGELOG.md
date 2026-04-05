# Changelog — coding-adventures-reed-solomon (Lua)

All notable changes to the Lua `reed-solomon` package are documented here.

## [0.1.0] — 2026-04-05

### Added

- Initial implementation of `coding_adventures.reed_solomon` (MA02).
- `build_generator(n_check)` — builds the monic RS generator polynomial
  g(x) = (x+α¹)(x+α²)···(x+α^{n_check}) in little-endian coefficient form.
  Cross-language test vector: `build_generator(2)` → `{8, 6, 1}`.
- `encode(message, n_check)` — systematic RS encoding. Pads the message with
  n_check zeros, divides by the generator polynomial (big-endian synthetic
  division), and appends the remainder as check bytes.
- `syndromes(received, n_check)` — evaluates the received polynomial at each
  root α^j (j=1..n_check) of the generator using big-endian Horner's method.
  Returns a table of n_check syndrome values; all-zero means no errors.
- `error_locator(syndromes)` — Berlekamp-Massey LFSR algorithm. Finds the
  shortest linear recurrence (error locator polynomial Λ) that generates the
  syndrome sequence. Returns Λ in little-endian form with Λ[1]=1.
- `decode(received, n_check)` — full 5-step RS decode pipeline:
  1. Syndrome computation
  2. Berlekamp-Massey → Λ(x)
  3. Chien search → error positions (exhaustive evaluation of Λ at all n positions)
  4. Forney algorithm → Ω(x) = S(x)·Λ(x) mod x^{n_check}, formal derivative Λ'(x),
     and magnitude e_p = Ω(X_p⁻¹)/Λ'(X_p⁻¹) for each error position p
  5. XOR magnitudes at error positions; return first k message bytes
- Internal helpers: `poly_eval_be`, `poly_eval_le`, `poly_mul_le`, `poly_mod_be`,
  `inv_locator`, `chien_search`, `forney` — all with Knuth-style literate comments.
- Error conventions:
  - `error("InvalidInput: ...")` — bad n_check (0 or odd), total > 255, or
    received shorter than n_check
  - `error("TooManyErrors: ...")` — correction capacity exceeded
- 46 busted unit tests covering: generator cross-vector, root verification,
  encode systematic property, syndromes-of-codeword-zero, decode with 0/1/2/4
  errors, round-trip property, TooManyErrors detection, and all public API exports.
- Knuth-style literate programming throughout: worked examples, truth tables,
  field-theory explanations, and step-by-step algorithm traces.
