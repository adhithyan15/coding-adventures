# Changelog — reed-solomon

All notable changes to this package are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

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
