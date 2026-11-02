# x509-ocsp-rsa-pkcs1-v1-5-sha384-signature

Exact repository-owned RSA PKCS #1 v1.5 SHA-384 OCSP signature verification.

The crate accepts one structurally decoded `BasicOCSPResponse`, one separately
policy-validated `ValidatedRsaPublicKey`, and one explicit RSA public-operation
limit. It then:

1. requires the exact `sha384WithRSAEncryption` OID with present canonical NULL
   parameters;
2. rejects non-byte-aligned signature BIT STRINGs;
3. hashes the response's preserved canonical `ResponseData` DER with the
   repository-owned SHA-384 implementation;
4. applies the bounded repository-owned RSA public operation; and
5. validates the complete PKCS #1 v1.5 SHA-384 encoded message.

Success returns non-copyable evidence bound to the exact response object and
validated key. The crate does not route algorithms, select or authorize the
responder key, match `ResponderID`, bind `CertID` or status to a certificate or
path, apply certificate role or extension policy, decide freshness or
revocation, establish trust, implement TLS or HTTP, or perform network effects.
It therefore does not make concrete OAuth HTTPS or live-provider use ready.

This pure public-data verifier performs no credential access, signing, storage,
clock acquisition, or external effect, so it emits no runtime audit event.

## Development

```bash
cargo test -p x509-ocsp-rsa-pkcs1-v1-5-sha384-signature --all-targets
cargo clippy -p x509-ocsp-rsa-pkcs1-v1-5-sha384-signature --all-targets -- -D warnings
```
