# x509-ocsp-response

Allocation-free structural decoding for one RFC 6960 `OCSPResponse` envelope:

- the assigned `OCSPResponseStatus` values 0, 1, 2, 3, 5, and 6;
- one required terminal `[0] EXPLICIT ResponseBytes` for a successful status;
- no response bytes for any assigned error status;
- one generic response-type OBJECT IDENTIFIER and one borrowed OCTET STRING;
  and
- exact complete response bytes plus shared DER depth, element, length, and
  OID-arc limits.

The response-type OID is data, not authority. This crate does not recognize
`id-pkix-ocsp-basic` or decode the OCTET STRING as a `BasicOCSPResponse`.
Higher layers must perform that dispatch explicitly and retain the same bounded
decoder authority.

## Security boundary

This crate parses outer syntax only. It does not select or enable an algorithm,
verify a signature, authorize a responder, bind status to a certificate or
path, interpret extensions, compare times to a clock, decide revocation, fetch
or cache data, or provide HTTP, TLS, OAuth transport, or network access.

Run the package tests from the Rust workspace:

```sh
cargo test -p x509-ocsp-response -- --nocapture
```
