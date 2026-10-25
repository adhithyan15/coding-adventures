# x509-ocsp-cert-id-binding

Data-driven binding of one OCSP `SingleResponse` `CertID` to exact target and
issuer certificates.

The crate accepts one structurally decoded `SingleResponse`, one exact target
certificate, one exact candidate issuer certificate, and caller-owned hash
policy. It then:

1. requires the target certificate's canonical serial number to match the
   `CertID` serial;
2. invokes hash policy with the exact `CertID` algorithm and the issuer
   certificate's canonical subject Name DER;
3. invokes the same policy with that algorithm and the issuer certificate's
   `subjectPublicKey` BIT STRING contents; and
4. compares both complete, caller-sized digests with the repository-owned
   constant-time equality primitive.

The digest length is public, fixed at compile time, shared by both identifier
fields, and must be non-zero. The caller remains responsible for selecting and
implementing an allowed hash algorithm. Success returns non-copyable evidence
bound to the exact response, target, and issuer objects.

This crate does not authorize the selected hash, prove that the issuer signed
the target, verify or authorize an OCSP responder, authenticate a certificate
path, interpret status or extensions, decide freshness or revocation, read a
clock, implement TLS or HTTP, or perform network effects. It therefore does not
make concrete OAuth HTTPS or live-provider use ready.

## Development

```bash
cargo test -p x509-ocsp-cert-id-binding --all-targets
cargo clippy -p x509-ocsp-cert-id-binding --all-targets -- -D warnings
```
