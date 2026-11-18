# rsa-pss-sha512-signature

Bounded repository-owned SHA-512 RSASSA-PSS signature verification.

## What it does

This crate composes the repository-owned RSA public operation with the fixed
SHA-512, MGF1-SHA-512, 64-byte-salt EMSA-PSS validator. Callers provide a
previously validated RSA public key, an exact modulus-width signature, an
already-computed 64-byte message digest, exact DB scratch, and independent
public work ceilings. Successful borrowed evidence binds those exact public
inputs.

It does not hash messages, parse RSA-PSS parameters or certificates, select an
issuer or algorithm, build a certificate path, choose trust roots, check time,
hostnames, or revocation, process TLS or HTTP, authorize OAuth transport, or
perform I/O. This package alone is not evidence of certificate, HTTPS, or
live-provider readiness.

## Example

```rust
use rsa_pss_sha512_signature::{
    verify_rsa_pss_sha512_signature, RsaPssSha512SignatureLimits,
};

# fn verify(
#     key: x509_rsa_public_key::ValidatedRsaPublicKey<'_>,
#     signature: &[u8],
#     digest: &[u8; 64],
#     scratch: &mut [u8],
# ) -> Result<(), Box<dyn std::error::Error>> {
let limits = RsaPssSha512SignatureLimits::new(512, 512)?;
let evidence =
    verify_rsa_pss_sha512_signature(key, signature, digest, scratch, limits)?;
assert_eq!(evidence.signature(), signature);
# Ok(())
# }
```

## Dependencies

- rsa-pss-sha512
- rsa-public-operation
- x509-rsa-public-key

## Development

```bash
# Run tests
bash BUILD
```
