# SSS01 — Shamir's Secret Sharing

## Overview

A from-scratch implementation of Shamir's Secret Sharing over GF(2^8).
Lives at the same layer as the other crypto leaves
(`chacha20-poly1305`, `argon2id`, `csprng`, `ed25519`) and is consumed
by higher Vault layers.

This document specifies the public API, the field choice, the wire
format, the security properties, and the deliberate non-goals. The
implementation lives at `code/packages/rust/shamir/`.

## Why this primitive exists

A vault is unsealed by some "key material." If the only thing that
holds that key material is one human's password, the vault has a
single point of failure: a typo, a forgotten password, an under-the-
table coercion, or a death. Shamir's Secret Sharing turns "one secret"
into `n` shares such that any `k` of them reconstruct it, and no
`k − 1` of them reveal anything.

The Vault stack uses this in three places:

- **VLT03 — `DistributedFragmentCustodian`**. Vault KEK split into
  shares held by N operators. K operators must collaborate to
  unseal. (HashiCorp Vault's quorum-unseal model.)
- **VLT05 — `ShamirQuorumAuthenticator`**. Same shape, but as an
  authentication factor that contributes key material to unlock
  derivation rather than just gating access.
- **Emergency / family / team recovery flows**. Bitwarden Emergency
  Access, 1Password Family / Team Recovery, Proton Account Recovery
  — all use threshold schemes underneath.

## Field

We work in GF(2^8), the finite field with 256 elements, under the
AES reduction polynomial `X^8 + X^4 + X^3 + X + 1` (= 0x11B).
Generator `g = 3`.

**Why GF(2^8):**

- One byte per field element — no padding overhead for byte secrets.
- Addition and subtraction are both XOR.
- Multiplication via 256-byte log/exp tables under the AES poly. The
  same tables AES uses for MixColumns; tested against published
  reference vectors (e.g. `0x02 · 0x87 = 0x15`).
- 256 elements = enough room for 255 distinct share indices, which
  bounds `n ≤ 255`. Higher shareholders need a larger field
  (GF(2^16) is the obvious next step); we don't need it for any
  realistic vault use case.

**Why generator 3:** 3 is a primitive element under 0x11B (its
powers cycle through all 255 non-zero elements). The table builder
uses the algebraic identity `3 = X + 1` so `3·x = 2·x + x`, where
`2·x` is a left-shift-with-conditional-XOR-of-the-poly.

## Public API

```rust
pub struct Share {
    pub x: u8,        // share index, 1..=255 (0 reserved for the secret)
    pub y: Vec<u8>,   // one byte per byte of the original secret
}

pub fn split(secret: &[u8], k: usize, n: usize) -> Result<Vec<Share>, ShamirError>;
pub fn combine(shares: &[Share]) -> Result<Vec<u8>, ShamirError>;

impl Share {
    pub fn encode(&self) -> Vec<u8>;                       // x || y
    pub fn decode(bytes: &[u8]) -> Result<Self, ShamirError>;
}

pub enum ShamirError {
    InvalidThreshold { k: usize, n: usize },
    EmptySecret,
    BelowThreshold,
    InconsistentShares,
    InvalidShare,
    Csprng(coding_adventures_csprng::CsprngError),
}
```

### `split(secret, k, n)`

Preconditions enforced as `Err`:

- `1 ≤ k ≤ n ≤ 255` — else `InvalidThreshold`.
- `secret.len() ≥ 1` — else `EmptySecret`.

Algorithm:

1. For each byte `s` of the secret:
   - Build a polynomial of degree `k − 1`: `f(X) = a_{k-1}·X^{k-1} +
     … + a_1·X + a_0` with `a_0 = s` and `a_1, …, a_{k-1}` drawn
     uniformly at random from GF(2^8) via the OS CSPRNG.
   - Evaluate `f` at every `x ∈ {1, 2, …, n}` using Horner's method.
2. Bundle the per-byte y-values: share `i` gets `(i, [y_i_byte0,
   y_i_byte1, …])`.

Polynomial coefficients are wrapped in `Zeroizing<Vec<u8>>` so they
are wiped on early-return error paths *and* on natural drop.

### `combine(shares)`

Validation:

- `shares.len() ≥ 1` — else `BelowThreshold`.
- All shares have the same `y.len()` — else `InconsistentShares`.
- All `share.x` are distinct (else Lagrange divides by zero).
- No `share.x == 0` (0 is reserved for the secret position).

Reconstruction: Lagrange interpolation at `x = 0`, byte-by-byte:

```text
   f(0) = Σ_i  y_i · ∏_{j ≠ i}  x_j / (x_i ⊕ x_j)         (in char 2, − = ⊕)
```

`combine` does **not** know the original threshold `k`. It just runs
Lagrange on whatever shares it is given. With fewer than `k` shares,
the output is a uniformly distributed byte string of the right
length — there is no way to detect insufficient input from the bytes
alone (this is what "information-theoretic security" means).

## Wire format

```text
   share_bytes = x_byte || y_bytes
     x_byte  : u8, 1..=255
     y_bytes : [u8; secret_len]
```

`Share::encode` produces this; `Share::decode` consumes it.

A share is exactly `1 + secret_len` bytes. There is no length prefix,
checksum, or identifier — those belong to higher layers if needed.

## Security properties

- **Information-theoretic security.** With `k − 1` shares, every
  possible secret of length `secret.len()` is exactly equally
  consistent with the observed shares. An adversary with infinite
  computing power gains nothing from `k − 1` shares.
- **Post-quantum safe by construction.** The scheme does not rely on
  any computational hardness assumption.
- **Forward secrecy by re-share.** Re-running `split` produces a
  fresh polynomial; old shares no longer combine with new ones. So
  rotating shares is a re-split.

## Deliberate non-goals

- **No authentication of shares.** A malicious shareholder who flips
  one byte of their share will cause `combine` to silently produce
  garbage. Tested explicitly: `tampered_share_silently_produces_garbage`.
  If you need verifiable shares, wrap with a Feldman or Pedersen
  scheme at a higher layer.
- **No threshold > 255 or share count > 255.** Bounded by GF(2^8).
- **No length-padding.** A secret of length `L` produces shares of
  length `L + 1`. If you don't want `L` to be visible to share
  holders, pad before splitting.
- **No share identifiers / labels / metadata.** Higher layers
  (recipient stores, audit logs) handle that.

## Error-message hardening

`ShamirError`'s `Display` impl uses only string literals — no
attacker-controlled bytes (e.g. share content) ever appear in error
output. This matches the discipline established by VLT01.

## Threat model & test coverage

| Threat                                                  | Defence                                                 | Test                                                           |
|---------------------------------------------------------|---------------------------------------------------------|----------------------------------------------------------------|
| Adversary holds `k − 1` shares                          | Information-theoretic; the coefficient is uniform       | `k_minus_one_shares_do_not_equal_secret` (smoke);  proof above |
| Caller passes nonsensical `(k, n)`                      | Up-front validation                                     | `split_rejects_k_zero` / `…_k_greater_than_n` / `…_n_over_255` |
| Caller passes empty secret                              | Up-front validation                                     | `split_rejects_empty_secret`                                   |
| Two shares accidentally have the same `x`               | `combine` rejects                                       | `combine_rejects_duplicate_x`                                  |
| Share with `x = 0`                                      | `combine` rejects (would interpolate to itself)         | `combine_rejects_x_zero`                                       |
| Length mismatch between shares                          | `combine` rejects                                       | `combine_rejects_inconsistent_lengths`                         |
| Empty share list                                        | `combine` rejects                                       | `combine_rejects_empty_share_list`                             |
| Share content leaked via `Debug` / `panic!`             | `Debug` redacts `y`                                     | `share_debug_does_not_leak_y`                                  |
| Share material lingering in memory after drop           | `Drop` zeroes `y`                                       | (covered by `Zeroize` / sibling `zeroize` crate's tests)       |
| Polynomial coefficients lingering after `split` returns | `Zeroizing<Vec<u8>>` wraps them at allocation site      | (covered by `Zeroize` / sibling `zeroize` crate's tests)       |

Plus 12 round-trip tests across various `(k, n)` combinations and
field-axiom tests (28 total).

## Citations

- Shamir, A. (1979). *How to share a secret*. Communications of the
  ACM 22(11), 612–613.
- FIPS 197 — Advanced Encryption Standard, §4 (the Galois field
  `GF(2^8)` and the reduction polynomial).
- HashiCorp Vault internals — "Quorum Unseal" docs.
- VLT00-vault-roadmap.md — VLT03 `DistributedFragmentCustodian` and
  VLT05 `ShamirQuorumAuthenticator`.
