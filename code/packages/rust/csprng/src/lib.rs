//! # coding_adventures_csprng — thin wrapper around the OS CSPRNG
//!
//! ## Why a wrapper
//!
//! Every secret the Vault generates (master salt, channel master keys,
//! nonces, session-token opaque IDs, the 24-byte XChaCha20 nonce) must
//! come from a **cryptographically secure** random source — not the
//! standard `rand::thread_rng()` (which is a userspace PRNG seeded
//! once) and not `rand::random()` in contexts where its CSPRNG
//! guarantees are not load-bearing.
//!
//! The right source is the operating system's kernel entropy pool:
//!
//!   * **Linux**: the `getrandom(2)` syscall (preferred), falling back
//!     to `/dev/urandom`.
//!   * **macOS / iOS**: `getentropy(2)`.
//!   * **Windows**: `BCryptGenRandom(…, BCRYPT_USE_SYSTEM_PREFERRED_RNG)`.
//!   * **FreeBSD/OpenBSD**: `getrandom`/`getentropy`.
//!
//! The [`getrandom`] crate is a single, well-audited shim over all of
//! these. We wrap it, not re-implement it, because the FFI dance is
//! large, boring, and already solved.
//!
//! This crate **adds** to `getrandom`:
//!
//!   * A tiny, stable, typed API (`CsprngError`) — callers don't touch
//!     the getrandom error type directly and don't have to deal with
//!     its `#[non_exhaustive]` variants.
//!   * `random_array::<N>()` for the common fixed-size case.
//!   * `random_u64()` / `random_u32()` helpers so callers don't have
//!     to hand-assemble integers from byte buffers.
//!   * An explicit zero-length guard so callers get a clear error on a
//!     programming mistake rather than a silent "success" returning an
//!     empty buffer.
//!   * Documentation that spells out the threat model and the trust
//!     boundary. This is the **one** capability in the Vault stack that
//!     must go outside the pure-crypto sandbox.
//!
//! ## What this crate does NOT do
//!
//! * **Does not re-seed or mix** the OS pool. The kernel is trusted.
//! * **Does not cache** entropy. Every call is a fresh syscall. For
//!   very high throughput callers, a ChaCha20-based stream expander
//!   seeded from this crate would be the right shape — but until we
//!   have that load, bypassing the syscall is a premature optimisation.
//! * **Does not zeroize** the returned buffer. Callers that need that
//!   should wrap the result in `coding_adventures_zeroize::Zeroizing`.
//!   We avoid a hard zeroize dependency here so this crate can be
//!   pulled into any sibling package without also pulling in the
//!   zeroize crate.
//!
//! ## Usage
//!
//! ```no_run
//! use coding_adventures_csprng::{random_bytes, random_array, random_u64};
//!
//! // Fresh 32-byte master key.
//! let key: [u8; 32] = random_array().expect("OS CSPRNG unavailable");
//!
//! // Dynamic length (e.g. for a caller-supplied tag length).
//! let salt = random_bytes(16).expect("OS CSPRNG unavailable");
//!
//! // 64-bit opaque lease ID (caller may still encode as UUID).
//! let lease_id = random_u64().expect("OS CSPRNG unavailable");
//! ```

#![deny(unsafe_code)]

// === Section 1. Error type ==================================================

/// Errors returnable from this crate.
#[derive(Debug)]
pub enum CsprngError {
    /// The OS CSPRNG could not be read. This is essentially never
    /// observed on a healthy system — it means the kernel entropy
    /// source is unavailable (very early boot, a sandboxed
    /// environment that denies the syscall, a corrupt `/dev/urandom`
    /// setup, etc.).
    ///
    /// The embedded string is the OS-level error message; callers
    /// should log it and fail closed rather than fall back to a
    /// weaker source.
    OsRandomUnavailable(String),

    /// The caller asked for a zero-length buffer. We surface this as
    /// an error rather than return an empty `Vec` because it almost
    /// always indicates a bug (a length that should have been
    /// validated earlier).
    ZeroLengthRequest,
}

impl core::fmt::Display for CsprngError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OsRandomUnavailable(msg) => {
                write!(f, "OS CSPRNG unavailable: {}", msg)
            }
            Self::ZeroLengthRequest => {
                write!(f, "zero-length random request (likely a caller bug)")
            }
        }
    }
}

impl std::error::Error for CsprngError {}

// === Section 2. Byte-buffer entry points ====================================

/// Fill `buf` with cryptographically secure random bytes from the OS.
///
/// On success `buf` is entirely overwritten. On error the buffer is
/// left in whatever state `getrandom` produced — callers that cannot
/// tolerate partial writes should wrap a fresh local buffer, copy on
/// success only, and rely on the caller's own zeroize policy.
pub fn fill_random(buf: &mut [u8]) -> Result<(), CsprngError> {
    if buf.is_empty() {
        return Err(CsprngError::ZeroLengthRequest);
    }
    getrandom::getrandom(buf).map_err(|e| CsprngError::OsRandomUnavailable(e.to_string()))
}

/// Allocate a `Vec<u8>` of length `n` and fill it with OS random bytes.
///
/// Callers handling secrets should immediately wrap the result in a
/// zeroize-on-drop container (e.g. `coding_adventures_zeroize::Zeroizing`).
pub fn random_bytes(n: usize) -> Result<Vec<u8>, CsprngError> {
    if n == 0 {
        return Err(CsprngError::ZeroLengthRequest);
    }
    let mut out = vec![0u8; n];
    fill_random(&mut out)?;
    Ok(out)
}

// === Section 3. Fixed-size and integer helpers ==============================

/// Return a fresh `[u8; N]` drawn from the OS CSPRNG.
///
/// Prefer this over `random_bytes(N)` when the length is a compile-
/// time constant — it lets the caller store the key on the stack and
/// avoids a heap allocation.
pub fn random_array<const N: usize>() -> Result<[u8; N], CsprngError> {
    if N == 0 {
        return Err(CsprngError::ZeroLengthRequest);
    }
    let mut out = [0u8; N];
    fill_random(&mut out)?;
    Ok(out)
}

/// Return a 64-bit unsigned integer drawn from the OS CSPRNG.
///
/// The bytes are read little-endian because that's what every
/// downstream consumer in this repo expects (`u64::from_le_bytes`).
/// For a 128-bit opaque ID, call this twice.
pub fn random_u64() -> Result<u64, CsprngError> {
    let bytes: [u8; 8] = random_array()?;
    Ok(u64::from_le_bytes(bytes))
}

/// Return a 32-bit unsigned integer drawn from the OS CSPRNG.
pub fn random_u32() -> Result<u32, CsprngError> {
    let bytes: [u8; 4] = random_array()?;
    Ok(u32::from_le_bytes(bytes))
}

// === Section 4. Tests =======================================================
//
// Testing a CSPRNG is by its nature statistical, not deterministic.
// We verify:
//   1. Calls return buffers of the requested size.
//   2. Two back-to-back calls are overwhelmingly likely to differ
//      (collision in 32 random bytes has probability 2^-256; if it
//      happens, the test will rerun and it will not happen again).
//   3. The all-zero buffer is not returned (same argument — 2^-256
//      probability).
//   4. Length-0 requests are rejected with `ZeroLengthRequest`.
//
// We DO NOT try to statistically test randomness quality here. That
// belongs in the OS / the `getrandom` crate's test suite, not ours —
// and a real randomness test needs millions of bytes plus a NIST-
// suite-shaped battery, which is out of scope.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_bytes_returns_requested_length() {
        for n in [1, 7, 16, 32, 48, 1024] {
            let v = random_bytes(n).expect("OS CSPRNG should be available");
            assert_eq!(v.len(), n);
        }
    }

    #[test]
    fn random_bytes_rejects_zero_length() {
        match random_bytes(0) {
            Err(CsprngError::ZeroLengthRequest) => {}
            other => panic!("expected ZeroLengthRequest, got {:?}", other),
        }
    }

    #[test]
    fn fill_random_overwrites_buffer() {
        let mut buf = [0u8; 64];
        fill_random(&mut buf).expect("OS CSPRNG should be available");
        // The probability of 64 all-zero bytes is 2^-512 — if this
        // test flakes, the universe has bigger problems than our CI.
        assert!(buf.iter().any(|&b| b != 0));
    }

    #[test]
    fn fill_random_rejects_empty_buffer() {
        let mut empty: [u8; 0] = [];
        match fill_random(&mut empty) {
            Err(CsprngError::ZeroLengthRequest) => {}
            other => panic!("expected ZeroLengthRequest, got {:?}", other),
        }
    }

    #[test]
    fn two_calls_return_different_buffers() {
        // Probability of a collision in 32 bytes = 2^-256.
        let a = random_bytes(32).unwrap();
        let b = random_bytes(32).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn random_array_fixed_size() {
        let a: [u8; 16] = random_array().unwrap();
        let b: [u8; 16] = random_array().unwrap();
        assert_ne!(a, b);
        assert_eq!(a.len(), 16);
    }

    #[test]
    fn random_array_rejects_zero_length() {
        match random_array::<0>() {
            Err(CsprngError::ZeroLengthRequest) => {}
            other => panic!("expected ZeroLengthRequest, got {:?}", other),
        }
    }

    #[test]
    fn random_u64_yields_distinct_values() {
        let a = random_u64().unwrap();
        let b = random_u64().unwrap();
        // Probability of collision = 2^-64 ≈ 5e-20. Safe.
        assert_ne!(a, b);
    }

    #[test]
    fn random_u32_yields_distinct_values() {
        // Collision probability = 2^-32 ≈ 2e-10. Still fine.
        let a = random_u32().unwrap();
        let b = random_u32().unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn error_display_is_informative() {
        let err = CsprngError::ZeroLengthRequest;
        let s = format!("{}", err);
        assert!(s.contains("zero-length"));

        let err = CsprngError::OsRandomUnavailable("mock reason".into());
        let s = format!("{}", err);
        assert!(s.contains("OS CSPRNG"));
        assert!(s.contains("mock reason"));
    }

    #[test]
    fn many_calls_do_not_repeat_first_byte_deterministically() {
        // Very weak smoke test: over 256 draws of one byte we should
        // see more than one distinct value. This catches the "always
        // returns 0" failure mode without making claims about
        // distribution.
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        for _ in 0..256 {
            let v = random_bytes(1).unwrap();
            seen.insert(v[0]);
        }
        assert!(
            seen.len() > 1,
            "CSPRNG returned the same byte 256 times in a row — broken source"
        );
    }
}
