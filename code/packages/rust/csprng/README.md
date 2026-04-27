# coding_adventures_csprng

Thin wrapper around the operating-system cryptographic random source.

## What It Does

Provides a typed, explicit API over the OS CSPRNG — `getrandom(2)` on
Linux, `getentropy(2)` on macOS, `BCryptGenRandom` on Windows, via
the `getrandom` crate shim:

- `fill_random(&mut [u8]) -> Result<(), CsprngError>`
- `random_bytes(n: usize) -> Result<Vec<u8>, _>`
- `random_array::<N>() -> Result<[u8; N], _>` (fixed-size stack-allocated)
- `random_u64() / random_u32()` — little-endian u64/u32 helpers

All four reject zero-length requests with `CsprngError::ZeroLengthRequest`
(almost always a caller bug) rather than returning silently-empty output.

## Why A Wrapper

The `rand` and `getrandom` crates are large and expose several
overlapping APIs. For the D18 Chief-of-Staff Vault stack, we want a
single, tiny, audited entry point that:

- Makes the trust boundary explicit: "this is **the** call that goes
  to the OS."
- Returns a stable local error type so call sites don't import
  `getrandom`'s `#[non_exhaustive]` `Error`.
- Gives fixed-size integer and array helpers so callers don't
  hand-assemble bytes.
- Documents what this crate does *not* do (no pool mixing, no
  user-space caching, no zeroization).

## What It Does NOT Do

- **Does not re-seed or mix** the OS entropy pool. The kernel is
  trusted as the sole source.
- **Does not cache or stream**. Every call is a fresh syscall. For a
  high-throughput keystream we'd want a ChaCha20 expander seeded from
  this; we don't have that load yet, so we don't build it yet.
- **Does not zeroize** the returned buffer. Callers handling secrets
  should wrap the result in
  `coding_adventures_zeroize::Zeroizing`. We keep the zeroize
  dependency out of this crate so non-secret callers (nonces, salts,
  opaque IDs) don't pay for it.
- **Not a fallback chain.** If the OS CSPRNG is unavailable we return
  `OsRandomUnavailable(…)` — callers must fail closed, not fall back
  to a weaker source.

## Usage

```rust
use coding_adventures_csprng::{random_bytes, random_array, random_u64};

// Fresh 32-byte master key (stack-allocated).
let key: [u8; 32] = random_array().expect("OS CSPRNG unavailable");

// 24-byte XChaCha20 nonce.
let nonce: [u8; 24] = random_array().expect("OS CSPRNG unavailable");

// Dynamic length (salt sized by a caller-supplied parameter).
let salt = random_bytes(16).expect("OS CSPRNG unavailable");

// 64-bit opaque lease ID.
let lease_id = random_u64().expect("OS CSPRNG unavailable");
```

## How It Fits

Part of the D18 Chief-of-Staff Vault crypto stack. Every Vault secret
that is *generated* (rather than derived from the user's passphrase)
flows through this crate:

- Vault master salt (Argon2id input).
- Channel master keys (CMKs).
- Per-message nonces for XChaCha20-Poly1305 encryption-at-rest.
- Opaque lease IDs.
- UUID v4/v7 random payloads (delegated from `coding_adventures_uuid`).

## Capability

This crate requires the `os_random` capability — it is the **only**
capability-taking crate in the Vault crypto stack. Everything else
(ChaCha20, Poly1305, Argon2id, BLAKE2b, ct-compare, zeroize) is pure
computation.

## Implementation Notes

- Wraps the `getrandom` crate; we do not re-implement the per-OS FFI.
- `#![deny(unsafe_code)]` — zero unsafe in this wrapper.
- Explicit `ZeroLengthRequest` error for zero-length inputs.
- Error type wraps the OS-level message as a `String` so callers can
  log it. Does not leak file paths or other sensitive context.
