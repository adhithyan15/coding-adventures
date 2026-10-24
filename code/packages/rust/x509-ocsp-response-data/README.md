# x509-ocsp-response-data

Allocation-free structural decoding for one RFC 6960 `ResponseData` value:

- DER's default v1 version is accepted only by omission;
- explicit `byName` and `byKey` responder identifiers;
- strict OCSP GeneralizedTime spelling for `producedAt`;
- a non-empty, fixed-capacity, ordered collection of structural `SingleResponse` values;
- one optional `[1] EXPLICIT responseExtensions` field; and
- exact borrowed canonical DER bytes for later signature verification;
- shared DER depth, element, length, OID-arc, extension-count, and response-count limits.

The crate is a syntax component, not an OCSP decision engine. It does not interpret responder key hashes or extensions, select hash algorithms, bind responses to certificates, authenticate a response, authorize a responder, compare timestamps to a clock, apply revocation policy, or provide HTTP/TLS.

Run the package tests from the Rust workspace:

```sh
cargo test -p x509-ocsp-response-data -- --nocapture
```
