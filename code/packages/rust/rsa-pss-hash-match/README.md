# rsa-pss-hash-match

Bounded, data-driven matching of a recomputed RFC 8017 EMSA-PSS `H'` value
against the encoded message's `H` field.

The caller supplies a selected digest length and comparison ceiling. The
validator requires both complete hashes at that exact length, consumes every
selected byte without revealing a mismatch position, and returns borrowed
non-copyable evidence when they match.

```rust
use rsa_pss_hash_match::{validate_rsa_pss_hash_match, RsaPssHashMatchPolicy};

let policy = RsaPssHashMatchPolicy::new(4, 64).unwrap();
let computed = [1, 2, 3, 4];
let encoded = [1, 2, 3, 4];
let matched = validate_rsa_pss_hash_match(&computed, &encoded, policy).unwrap();
assert_eq!(matched.hash_length(), 4);
```

## Boundary

This package only compares caller-provided bytes. It does not construct or hash
`M'`, run MGF1, validate a PSS data block, parse or operate on an RSA key, apply
RSAVP1, select an algorithm, or establish a signature verdict. It performs no
credential, signing, storage, clock, entropy, filesystem, network, or other
external effect. This API makes no constant-time guarantee; RSA-PSS signature
verification values are public.

## Development

```bash
bash BUILD
```
