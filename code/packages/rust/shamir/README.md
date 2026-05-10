# `coding_adventures_shamir`

Shamir's Secret Sharing over GF(2^8), from scratch.

## What it does

Split a byte string into `n` shares such that any `k` of them
reconstruct the original, and any `k − 1` of them reveal nothing.

```rust
use coding_adventures_shamir::{split, combine};

let master_key = b"my 32-byte master key for vault!";
let shares = split(master_key, /* k = */ 3, /* n = */ 5).unwrap();

// Hand shares[0], shares[2], shares[4] to three trusted humans / HSMs.
// Later, any three reconstruct:
let recovered = combine(&[
    shares[0].clone(),
    shares[2].clone(),
    shares[4].clone(),
]).unwrap();
assert_eq!(&recovered[..], master_key);
```

## Where it fits in the Vault stack

This crate is a **foundation primitive**, sitting beside other
crypto leaves like `chacha20-poly1305`, `argon2id`, `csprng`, and
`ed25519`. It is consumed by:

- **VLT03 — `vault-key-custody`** as the cryptographic core of the
  `DistributedFragmentCustodian`: split a vault's KEK into
  shares held by N operators, requiring K of them to unseal.
  Mirrors HashiCorp Vault's quorum-unseal model.
- **VLT05 — `vault-auth`** as the cryptographic core of the
  `ShamirQuorumAuthenticator`: K-of-N humans collectively
  contribute to unlock derivation.
- **Emergency / family / team recovery flows** (Bitwarden,
  1Password, Proton).

See [`VLT00-vault-roadmap.md`](../../../specs/VLT00-vault-roadmap.md)
for the full Vault layering.

## Properties

- **Information-theoretic security.** `k − 1` shares reveal nothing
  about the secret, even to an adversary with infinite computing
  power. (Standard fact about polynomials of degree `k − 1` over a
  finite field — fewer than `k` points leave the polynomial
  undetermined.)
- **Post-quantum safe by construction.** The scheme does not rely on
  any computational hardness assumption, so it cannot be broken by
  Shor or Grover.
- **No authentication.** A malicious shareholder who alters their
  share will cause reconstruction to silently produce garbage. If
  you need verifiable shares, wrap the output in a hash-and-MAC
  layer (or use a higher-level Vault crate that does so).

## Parameters

- `1 ≤ k ≤ n ≤ 255`. Threshold and share count are bounded by the
  number of non-zero elements in GF(2^8) (255).
- Secret length `≥ 1`. Each share is `1 + secret_len` bytes.
- Splitting cost: O(n · k · secret_len) field ops.
- Combining cost: O(k² · secret_len) field ops.

## Field choice

We work in GF(2^8) under the AES reduction polynomial
`X^8 + X^4 + X^3 + X + 1` (= 0x11B), generator 3. This is the same
field AES uses, so log/exp tables can be cross-checked against
published MixColumns vectors.

## Citations

- Shamir, A. (1979). _How to Share a Secret_. Communications of the
  ACM, 22(11), 612–613.
- FIPS 197 — Advanced Encryption Standard, §4 (the Galois field).
- HashiCorp Vault internals docs — quorum unseal.
