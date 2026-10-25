# x509-ocsp-responder-id-binding

Exact OCSP `ResponderID`-to-certificate binding.

The crate accepts one structurally decoded `BasicOCSPResponse`, one exact
decoded candidate responder certificate, and caller-owned Name equality
policy. It then:

1. invokes Name policy exactly once in responder-Name-to-certificate-subject
   order for `byName`; or
2. requires a 20-byte `byKey` value, hashes the certificate's
   `subjectPublicKey` BIT STRING contents with the repository-owned SHA-1
   implementation, and compares the complete digest with the repository-owned
   constant-time equality primitive.

SHA-1 here is the fixed RFC 6960 `KeyHash` identifier rule, not a certificate
or response signature algorithm. Success returns non-copyable evidence bound
to the exact response, exact certificate, and matched identifier kind.

The crate does not verify the response signature, prove that the certificate's
key signed it, authorize the responder, apply certificate role, EKU,
critical-extension, algorithm, path, or trust policy, bind a `CertID` or status
to a certificate or path, decide freshness or revocation, implement TLS or
HTTP, or perform network effects. It therefore does not make concrete OAuth
HTTPS or live-provider use ready.

## Development

```bash
cargo test -p x509-ocsp-responder-id-binding --all-targets
cargo clippy -p x509-ocsp-responder-id-binding --all-targets -- -D warnings
```
