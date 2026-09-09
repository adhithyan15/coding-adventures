# Changelog — reed-solomon

All notable changes to this package are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.2.0] — 2026-09-07

Added per `code/specs/MA04-qr-decoder.md` §2, to support decoding QR
Code's Reed-Solomon codewords (root base `b=0`) without reimplementing
Berlekamp-Massey/Chien-search/Forney a second time.

### Added

- **`build_generator_with_base(n_check, b)`** — like `build_generator`, but
  with an arbitrary root base `b` instead of the fixed `α¹`:
  `g(x) = ∏(x + α^{b+i})` for `i = 0..n_check`. `build_generator(n_check)`
  is now a thin wrapper calling this with `b=1`. Unlike `build_generator`,
  does **not** require `n_check` to be even — QR Code's own
  ECC-codewords-per-block table genuinely contains odd values (e.g. 7 at
  version 1 / ECC level L), and the even-only restriction was never a
  mathematical requirement of Reed-Solomon, just this crate's original API
  choice (preserved unchanged in the b=1-only wrapper).
- **`syndromes_with_base(received, n_check, b)`** — syndromes
  `Sᵢ = r(α^{b+i})` for `i = 0..n_check`. `syndromes(received, n_check)` is
  now a thin wrapper calling this with `b=1`.
- **`decode_with_base(received, n_check, b)`** — full decode pipeline
  parameterized on root base. `decode(received, n_check)` is now a thin
  wrapper calling this with `b=1`. Like `build_generator_with_base`, does
  not require even `n_check`.

### Fixed

- **Forney's error-magnitude formula was silently wrong for any base other
  than `b=1`.** The general Forney algorithm needs a base-dependent
  correction factor `Xₚ^{1-b}` that happens to equal `1` (a no-op) exactly
  at `b=1` — which is why the original, uncorrected formula was correct for
  every pre-existing test, all of which exercise only the implicit `b=1`
  default — but becomes the *forward* locator `Xₚ` itself at `b=0`.
  Without this factor, `decode_with_base(..., 0)` produced plausible-looking
  but numerically wrong "corrected" bytes instead of erroring — caught by
  a full single-error sweep across every codeword position at `b=0`, not
  by a handful of spot-check corruptions (which can pass by coincidence at
  small `t`).

### Testing

- 16 new tests covering the `_with_base` API: b=1-matches-default sanity
  checks, b=0 generator root verification, b=0 round-trip/corruption-
  recovery/beyond-capacity, a full single-error sweep across every
  codeword position at b=0, and real QR-shaped odd-`n_check` (7) coverage.
  All 53 pre-existing tests (the original public API's full suite) still
  pass unchanged, proving this is a purely additive, non-breaking release.

## [0.1.0] — 2026-04-04

### Added

- **`encode(message, n_check)`** — Systematic Reed-Solomon encoding over GF(256).
  Appends `n_check` check bytes to the message. The message bytes are preserved
  in the first `k = message.len()` positions of the output (systematic form).

- **`decode(received, n_check)`** — Full syndrome-based decoding pipeline:
  syndromes → Berlekamp-Massey → Chien search → Forney → byte correction.
  Corrects up to `t = n_check / 2` byte errors.

- **`syndromes(received, n_check)`** — Compute the `n_check` syndrome values
  `Sᵢ = r(αⁱ)` for `i = 1, …, n_check`. All-zero syndromes mean no errors;
  any non-zero syndrome signals corruption.

- **`build_generator(n_check)`** — Build the generator polynomial
  `g(x) = ∏(x + αⁱ)` for `i = 1..n_check` in little-endian GF(256) form.

- **`error_locator(syndromes)`** — Compute the error locator polynomial Λ(x)
  from a syndrome slice using the Berlekamp-Massey algorithm. Exposed as a
  public function for external tools (QR decoders, diagnostics).

- **`RSError`** enum with two variants:
  - `TooManyErrors` — codeword has more errors than `t`; unrecoverable.
  - `InvalidInput(String)` — `n_check` is odd/zero, or codeword exceeds
    the 255-byte GF(256) block size limit.

- **Comprehensive test suite** (`tests/reed_solomon_test.rs`):
  - Generator polynomial correctness and root verification
  - Encoding structural properties (systematic form, codeword length,
    zero-syndrome invariant)
  - Syndrome computation (zero on valid codeword, non-zero after corruption)
  - Round-trip encode → decode with zero errors
  - Error correction at every position in the codeword
  - Error correction up to capacity `t` for t = 1, 2, 3, 4, 10
  - `TooManyErrors` rejection for t+1 errors
  - Error locator polynomial degree checks
  - Concrete test vectors for reproducibility
  - Edge cases: empty message, single byte, all-zeros, all-ones,
    alternating bits, zero bytes in message, limit n=255
  - Input validation: odd n_check, zero n_check, oversized codeword

### Dependencies

- `gf256 = { path = "../gf256" }` (MA01) — all field arithmetic delegated here.
  No other dependencies. Zero unsafe code.

### Notes

- Internal polynomial convention:
  - **Little-endian** (index = degree): generator, error locator Λ, error
    evaluator Ω, syndrome polynomial S.
  - **Big-endian** (first = highest degree): codeword bytes, syndrome
    evaluation point order.
- Formal derivative in characteristic 2: only odd-index coefficients survive
  (`Λ'[j-1] = Λ[j]` for odd `j`; even terms vanish because 2 = 0 in GF(2^8)).
- Chien search uses `α^{255-i}` as the inverse of `α^i` (since `α^{255} = 1`).
  Special case: `i=0` → `α^{255 mod 255} = α^0 = 1` handled by the modulo.
- This is the Rust reference implementation. Future MA02 packages in TypeScript,
  Python, Go, Ruby, Elixir, Lua, Perl, and Swift will cross-validate test
  vectors against this crate.
