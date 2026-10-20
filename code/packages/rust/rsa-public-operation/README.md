# rsa-public-operation

A zero-external-dependency RFC 8017 RSA public-operation boundary for the
repository-owned X.509 verification stack.

The crate accepts only a `ValidatedRsaPublicKey` from `x509-rsa-public-key`, a
signature whose byte length exactly matches the normalized modulus, and a
caller-owned nonzero modulus-byte ceiling. It computes `signature^e mod n` and
returns the exact modulus-width big-endian encoded message, including any
leading zero octets.

```rust
use rsa_public_operation::{recover_encoded_message, RsaPublicOperationLimits};
# use x509_rsa_public_key::ValidatedRsaPublicKey;
# fn example(key: ValidatedRsaPublicKey<'_>, signature: &[u8]) {
let limits = RsaPublicOperationLimits::new(512).unwrap();
let encoded_message = recover_encoded_message(key, signature, limits).unwrap();
assert_eq!(encoded_message.len(), key.modulus().len());
# }
```

This crate is one arithmetic prerequisite, not a complete signature verifier.
It does not hash certificate bytes or interpret PKCS #1 v1.5/PSS encodings. The
returned bytes remain unauthenticated until a separate repository-owned
encoded-message validator checks them against a digest of the intended signed
bytes.

All inputs are public verification data. The implementation is deliberately
not a private-RSA API and makes no constant-time claim for secret operands.

## Validation

```bash
cargo test -p rsa-public-operation --all-targets
cargo clippy -p rsa-public-operation --all-targets -- -D warnings
```
