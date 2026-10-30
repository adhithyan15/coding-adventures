# x509-ocsp-rsa-pkcs1-v1-5-sha512-signature

Exact repository-owned RSA PKCS #1 v1.5 SHA-512 OCSP signature verification.

The crate accepts one structurally decoded `BasicOCSPResponse`, one separately
policy-validated `ValidatedRsaPublicKey`, and one explicit RSA public-operation
limit. It then:

1. requires the exact `sha512WithRSAEncryption` OID with present canonical NULL
   parameters;
2. rejects non-byte-aligned signature BIT STRINGs;
3. hashes the response's preserved canonical `ResponseData` DER with the
   repository-owned SHA-512 implementation;
4. applies the bounded repository-owned RSA public operation; and
5. validates the complete PKCS #1 v1.5 SHA-512 encoded message.

Success returns non-copyable evidence bound to the exact response object and
validated key. The crate does not route algorithms, select or authorize the
responder key, match `ResponderID`, bind `CertID` or status to a certificate or
path, apply certificate role or extension policy, decide freshness or
revocation, establish trust, implement TLS or HTTP, or perform network effects.
It therefore does not make concrete OAuth HTTPS or live-provider use ready.

## Example

```rust
use rsa_public_operation::RsaPublicOperationLimits;
use x509_ocsp_rsa_pkcs1_v1_5_sha512_signature::
    verify_x509_ocsp_rsa_pkcs1_v1_5_sha512_signature;

# fn verify(
#     response: &x509_ocsp_basic_response::X509OcspBasicResponse<'_>,
#     key: x509_rsa_public_key::ValidatedRsaPublicKey<'_>,
# ) -> Result<(), Box<dyn std::error::Error>> {
let evidence = verify_x509_ocsp_rsa_pkcs1_v1_5_sha512_signature(
    response,
    key,
    RsaPublicOperationLimits::new(512)?,
)?;
assert!(std::ptr::eq(evidence.response(), response));
# Ok(())
# }
```

## Development

```bash
cargo test -p x509-ocsp-rsa-pkcs1-v1-5-sha512-signature --all-targets
cargo clippy -p x509-ocsp-rsa-pkcs1-v1-5-sha512-signature --all-targets -- -D warnings
```
