# x509-ocsp-cert-id-encoding

Allocation-free encoding of one RFC 6960 OCSP `CertID` from caller-owned
validated DER and public hash bytes.

The crate validates one complete canonical hash `AlgorithmIdentifier` and one
complete canonical positive RFC 5280 certificate serial number under
caller-owned ASN.1 limits. It retains those exact inputs plus two caller-owned
issuer-hash byte strings, precomputes every nested DER length, and writes the
complete `CertID` only when caller-owned output is large enough. Validation and
capacity failures leave output untouched.

This crate does not choose or authorize a hash algorithm, compute a hash,
require a particular digest length, bind the values to certificates, prove an
issuer relationship, create a complete OCSP request, authenticate a responder,
decide revocation, read a clock, implement TLS or HTTP, or perform any network
effect. It does not make concrete OAuth HTTPS or live-provider use ready.

## Development

```bash
cargo test -p x509-ocsp-cert-id-encoding --all-targets
cargo clippy -p x509-ocsp-cert-id-encoding --all-targets -- -D warnings
```
