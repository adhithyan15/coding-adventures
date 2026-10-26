# SHA-384 (Rust)

Repository-owned SHA-384 from FIPS 180-4, implemented with fixed-size storage.
It is the digest prerequisite selected by the TLS 1.3
`TLS_AES_256_GCM_SHA384` cipher suite.

## API

```rust
use coding_adventures_sha384::{sha384, Sha384};

let one_shot = sha384(b"abc");

let mut streaming = Sha384::new();
streaming.update(b"a")?;
streaming.update(b"bc")?;
assert_eq!(streaming.digest(), one_shot);
# Ok::<(), coding_adventures_sha384::Sha384Error>(())
```

- `sha384(&[u8]) -> [u8; 48]` hashes one complete message.
- `Sha384::update(&[u8])` incrementally absorbs caller bytes.
- `Sha384::digest() -> [u8; 48]` takes a non-destructive snapshot.

The core allocates no heap memory. A streaming instance retains one 128-byte
partial block, eight 64-bit chaining words, and checked length state. Digest
snapshots use at most two 128-byte stack blocks for final padding. Retained and
temporary state is wiped with the workspace-owned volatile zeroization
primitive.

## Boundary

This crate is only a hash function. It does not implement HMAC, HKDF,
`HKDF-Expand-Label`, a TLS key schedule, transcript sequencing, cipher-suite
negotiation, AEAD, record protection, handshake processing, certificate
validation, trust, revocation, I/O, HTTP, OAuth transport, or network access.
It does not make the existing generic HKDF suitable for production TLS secret
handling and does not make concrete HTTPS available.

## Dependencies

Only the repository-owned `coding_adventures_zeroize` crate is used, to scrub
state and partial input bytes. There are no external production dependencies.
