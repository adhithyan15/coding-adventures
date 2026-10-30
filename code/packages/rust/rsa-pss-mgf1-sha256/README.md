# rsa-pss-mgf1-sha256

Allocation-free RFC 8017 MGF1 expansion for the SHA-256 RSA-PSS verification
path.

The caller supplies the exact 32-byte PSS hash value, caller-owned output, and
a validated nonzero work ceiling. The package computes each block as
`SHA-256(seed || I2OSP(counter, 4))`, with the counter starting at zero in
network byte order. Both the RFC maximum and the caller ceiling are checked
before any output byte changes.

```rust
use rsa_pss_mgf1_sha256::{generate_mgf1_sha256_mask, Mgf1Sha256Limits};

let seed = [0x42; 32];
let limits = Mgf1Sha256Limits::new(256).unwrap();
let mut mask = [0u8; 223];
generate_mgf1_sha256_mask(&seed, &mut mask, limits).unwrap();
```

## Boundary

This package is one deterministic byte-expansion primitive. It does not parse
RSA-PSS parameters or keys, choose a salt policy, validate an encoded message,
perform RSA arithmetic, verify a signature, process certificates, validate a
path or hostname, select trust roots, check revocation, run TLS, or open a
socket. Passing arbitrary bytes through MGF1 grants no cryptographic authority.

## Development

```bash
bash BUILD
```
