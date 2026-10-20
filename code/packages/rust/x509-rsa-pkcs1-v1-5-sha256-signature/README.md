# x509-rsa-pkcs1-v1-5-sha256-signature

An exact, repository-owned certificate-signature verifier for the RFC 8017
RSASSA-PKCS1-v1_5 scheme with SHA-256.

The crate accepts a complete `X509Certificate`, a separately policy-validated
issuer `ValidatedRsaPublicKey`, and an explicit RSA modulus-byte ceiling. It:

1. requires `sha256WithRSAEncryption` with present NULL parameters;
2. rejects non-byte-aligned RSA signature BIT STRINGs;
3. hashes the certificate's preserved `TBSCertificate` DER with the repository
   SHA-256 implementation;
4. applies the repository-owned bounded RSA public operation; and
5. validates the complete canonical PKCS #1 v1.5 SHA-256 encoded message.

Success returns non-copyable borrowed evidence that binds the exact certificate
and validated issuer key used by verification. The caller still owns
issuer selection and linkage, algorithm routing, RSA strength policy,
certificate roles, path construction and validation, trust roots, time,
server identity, revocation, TLS, HTTP, and every network effect. This crate is
therefore not a certificate validator or HTTPS implementation.

## Validation

```bash
cargo test -p x509-rsa-pkcs1-v1-5-sha256-signature --all-targets
cargo clippy -p x509-rsa-pkcs1-v1-5-sha256-signature --all-targets -- -D warnings
```
