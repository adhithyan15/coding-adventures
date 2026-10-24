# x509-ocsp-response

Allocation-free structural decoding for one RFC 6960 `OCSPResponse` envelope:

- the assigned `OCSPResponseStatus` values 0, 1, 2, 3, 5, and 6;
- one required terminal `[0] EXPLICIT ResponseBytes` for a successful status;
- no response bytes for any assigned error status;
- one generic response-type OBJECT IDENTIFIER and one borrowed OCTET STRING;
  and
- exact complete response bytes plus shared DER depth, element, length, and
  OID-arc limits.

The generic response-type OID remains data, not authority. A separate explicit
helper accepts only the exact `id-pkix-ocsp-basic` OID, requires the OCTET
STRING contents to be one canonical DER document, and composes the
repository-owned `BasicOCSPResponse` decoder through the same caller-owned
decoder. Unknown response types fail before their body is parsed. Dispatch
errors are payload-free and use offsets local to the embedded response body.

## Security boundary

This crate parses outer and explicitly dispatched basic-response syntax only.
It does not select or enable an algorithm, verify a signature, authorize a
responder, bind status to a certificate or path, interpret extensions, compare
times to a clock, decide revocation, fetch or cache data, or provide HTTP, TLS,
OAuth transport, or network access.

Run the package tests from the Rust workspace:

```sh
cargo test -p x509-ocsp-response -- --nocapture
```
