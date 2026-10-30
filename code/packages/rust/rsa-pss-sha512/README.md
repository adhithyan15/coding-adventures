# rsa-pss-sha512

Allocation-free RFC 8017 EMSA-PSS encoded-message validation for the fixed
SHA-512 and 64-byte-salt profile.

The caller supplies the complete recovered encoded message, public `emBits`,
the expected 64-byte message digest, exact data-block scratch storage, and a
validated encoded-message work ceiling. The package composes the repository's
public PSS layout gate and bounded SHA-512 MGF1 primitive, then validates the
complete unmasked DB and recomputed PSS hash.

```rust
use rsa_pss_sha512::{validate_rsa_pss_sha512_encoded_message, RsaPssSha512Limits};

let encoded_message = [0u8; 256];
let message_digest = [0u8; 64];
let mut data_block_scratch = [0u8; 191];
let limits = RsaPssSha512Limits::new(256).unwrap();
let _ = validate_rsa_pss_sha512_encoded_message(
    &encoded_message,
    2047,
    &message_digest,
    &mut data_block_scratch,
    limits,
);
```

## Boundary

This package does not parse RSA-PSS parameters or keys, perform RSA arithmetic,
bind recovered bytes to a signature or certificate, select a TLS signature
scheme, process a handshake, validate a path or hostname, select trust, check
revocation, perform I/O, authorize HTTP or OAuth transport, or open a socket.
An encoded-message verdict alone grants no signature, certificate, TLS, or
network authority. The public verification-byte comparison makes no
constant-time guarantee.

## Development

```bash
bash BUILD
```
