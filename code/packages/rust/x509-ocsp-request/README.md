# x509-ocsp-request

Allocation-free framing for the minimal unsigned RFC 6960 OCSP request shape
containing exactly one `Request` and one caller-supplied canonical `CertID`.

The crate first validates the complete identifier with the repository-owned
structural decoder. It then emits the exact nested `OCSPRequest`, `TBSRequest`,
`requestList`, and `Request` sequences into caller-owned output. Every DER
length and the complete output capacity are checked before output changes.

This narrow encoder deliberately omits the default version, requestor name,
per-request extensions, request extensions, and optional signature. It does
not choose or compute a hash, bind the identifier to certificates, add a
nonce, authorize a responder or destination, perform DNS, TLS, HTTP, caching,
or I/O, emit audit or trace events, access a credential, decide revocation,
authorize OAuth transport, or open a network connection. It does not make
concrete HTTPS or live-provider use ready.

## Validation

```bash
bash BUILD
cargo clippy -p x509-ocsp-request --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-ocsp-request --no-deps
```
