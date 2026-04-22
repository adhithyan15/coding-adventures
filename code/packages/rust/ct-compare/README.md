# coding_adventures_ct_compare

Constant-time byte-slice equality for Rust, implemented from scratch
— no external dependencies.

## What It Does

Provides data-independent equality primitives so MAC-tag, session-key,
and password-hash comparisons can't be broken by timing side-channels:

1. `ct_eq(a: &[u8], b: &[u8]) -> bool` — constant-time equality on
   byte slices.
2. `ct_eq_fixed::<N>(a: &[u8; N], b: &[u8; N]) -> bool` — type-safe
   fixed-length variant.
3. `ct_select_bytes(a, b, choice) -> Vec<u8>` — branchless select
   between two same-length slices.
4. `ct_eq_u64(a: u64, b: u64) -> bool` — constant-time 64-bit equality
   for counters and lease IDs.

## Why A Naive `==` Isn't Enough

```rust
for (x, y) in a.iter().zip(b.iter()) {
    if x != y { return false; }  // early exit leaks how many bytes matched
}
```

An attacker who can time this loop — over the network, in a colocated
tenant, on the same machine — learns the longest matching prefix and
can walk a candidate tag byte by byte. That's exactly how Keyczar's
HMAC verification, Rack's session cookies, Google's 2009 password
incident, and many historic CVEs were broken.

This crate replaces the early exit with an XOR accumulator:

```rust
let mut acc = 0u8;
for i in 0..a.len() { acc |= a[i] ^ b[i]; }
black_box(acc) == 0
```

Every iteration does the same work regardless of which byte differs,
and `black_box` stops the optimiser from reasoning about the loop and
folding it back into an early-exit equivalent.

## What It Does NOT Do

- **Cache-timing / branch-predictor / speculative-execution leaks** —
  those need hardware-level countermeasures.
- **Length hiding** — the public API compares lengths with `==` and
  treats length as public. Real secrets (MAC tags, HMAC outputs, AEAD
  tags, session keys, Argon2 tags) have fixed, publicly-known lengths.
- **Zeroization** — see `coding_adventures_zeroize`.

## Usage

```rust
use coding_adventures_ct_compare::{ct_eq, ct_eq_fixed, ct_eq_u64, ct_select_bytes};

// MAC verification
if !ct_eq(&computed_tag, &received_tag) {
    return Err("invalid tag");
}

// Fixed-size tags (preferred when the length is a compile-time constant)
let ok: bool = ct_eq_fixed(&tag_a, &tag_b);

// Lease-ID comparison
if ct_eq_u64(lease.id, requested.id) { /* ... */ }

// Branchless conditional select
let picked = ct_select_bytes(&option_a, &option_b, predicate);
```

## How It Fits

Part of the D18 Chief-of-Staff Vault crypto stack. Used by Poly1305
AEAD decryption, HMAC verification, Argon2id password verification,
and Vault unlock-key comparison.

Self-contained: no runtime dependencies. `#![deny(unsafe_code)]`.

## Implementation Notes

- `#[inline(never)]` on the public functions. Inlining them into
  callers could let the optimiser specialise per-caller in ways that
  reintroduce data-dependent behaviour.
- `black_box` is used on every boolean result. It is the
  standard-library primitive for "treat this value as opaque to the
  optimiser" — without it, a sufficiently aggressive pass could in
  principle detect that the loop is equivalent to a short-circuit.
- `ct_select_bytes` builds a byte mask `0u8.wrapping_sub(choice as u8)`
  (`0xFF` or `0x00`) and applies `b ^ ((a ^ b) & mask)` per byte. No
  conditional branches, no early exits.
- `ct_eq_u64` folds `a ^ b` into the low bit via
  `(diff | -diff) >> 63`, the classic "is-any-bit-set" trick.
