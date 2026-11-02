# x509-rsa-pkcs1-v1-5-sha512-signature

An exact, repository-owned certificate-signature verifier for the RFC 8017
RSASSA-PKCS1-v1_5 scheme with SHA-512.

The crate accepts a complete `X509Certificate`, a separately policy-validated
issuer `ValidatedRsaPublicKey`, and explicit RSA public-operation limits. It:

1. requires `sha512WithRSAEncryption` with present NULL parameters;
2. rejects non-byte-aligned RSA signature BIT STRINGs;
3. hashes the certificate's preserved `TBSCertificate` DER with the repository
   SHA-512 implementation;
4. applies the repository-owned bounded RSA public operation; and
5. validates the complete canonical PKCS #1 v1.5 SHA-512 encoded message.

Success returns non-copyable borrowed evidence that binds the exact certificate
and validated issuer key used by verification. The caller still owns issuer
selection and linkage, algorithm routing, RSA strength policy, certificate
roles, path construction and validation, trust roots, time, server identity,
revocation, TLS, HTTP, and every network effect. This crate is therefore not a
certificate validator or HTTPS implementation.

This pure public-data verifier does not access credentials, private keys,
signing authority, storage, clocks, or external systems, so it emits no runtime
credential or external-effect audit event.

## Validation

```bash
cargo test -p x509-rsa-pkcs1-v1-5-sha512-signature --all-targets
cargo clippy -p x509-rsa-pkcs1-v1-5-sha512-signature --all-targets -- -D warnings
```
