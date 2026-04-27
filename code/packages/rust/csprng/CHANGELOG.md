# Changelog

## 0.1.0 — 2026-04-20

- Initial release.
- `fill_random(&mut [u8])` — fill an arbitrary caller-provided buffer
  with OS CSPRNG bytes.
- `random_bytes(n)` — allocate and fill a `Vec<u8>` of length `n`.
- `random_array::<N>()` — stack-allocated `[u8; N]` for compile-time
  sizes.
- `random_u64()` / `random_u32()` — little-endian integer draws.
- `CsprngError` enum with `OsRandomUnavailable(String)` and
  `ZeroLengthRequest` variants. Implements `std::error::Error` and
  `Display`.
- All four entry points reject zero-length requests with
  `ZeroLengthRequest` — surfacing what is almost certainly a caller
  bug rather than silently succeeding.
- Delegates the per-OS syscall dance to the `getrandom` crate —
  `getrandom(2)` on Linux, `getentropy(2)` on macOS, `BCryptGenRandom`
  on Windows.
- `#![deny(unsafe_code)]` — zero unsafe in the wrapper itself.
- 11 unit tests covering length correctness, non-zero output,
  distinct-output sanity, error variants, and display formatting.
- Built for the D18 Chief-of-Staff Vault — completes the Rust-side
  Vault crypto unblocker set together with zeroize, ct-compare, and
  XChaCha20-Poly1305.
