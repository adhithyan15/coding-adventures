# rsa-pkcs1-v1-5-sha512

Allocation-free validation of the exact RFC 8017 EMSA-PKCS1-v1_5 encoded
message for SHA-512.

The caller supplies the complete modulus-width output from a separate RSA
public operation and the expected 64-byte digest. The crate accepts only
`00 01`, at least eight `ff` padding bytes, the zero separator, the canonical
DER SHA-512 `DigestInfo` with present NULL parameters, and the complete digest.

## Security boundary

This package performs no RSA arithmetic, hashing, key or certificate parsing,
algorithm routing, signature verdict, path validation, trust-root selection,
revocation, TLS, HTTP, OAuth authorization, credential access, signing, I/O,
or external effect. Arbitrary caller-supplied bytes do not become an
authenticated signature merely because their formatting is valid. A later
composition must bind this gate to the exact RSA public-operation result and
repository-owned SHA-512 digest for the exact signed bytes.

## Development

```bash
bash BUILD
cargo clippy -p rsa-pkcs1-v1-5-sha512 --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p rsa-pkcs1-v1-5-sha512 --no-deps
```
