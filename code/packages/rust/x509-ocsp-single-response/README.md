# x509-ocsp-single-response

Allocation-free structural decoding for one RFC 6960 `SingleResponse`:

- required `CertID`, `CertStatus`, and `thisUpdate` fields in schema order;
- ordered optional `[0] EXPLICIT nextUpdate` and `[1] EXPLICIT singleExtensions` fields;
- strict OCSP GeneralizedTime spelling;
- bounded, unique-OID generic extensions; and
- shared DER depth, element, length, OID-arc, and extension-count limits.

The crate is a syntax component, not an OCSP decision engine. It does not select hash algorithms, bind the `CertID` to a certificate, authenticate the response, authorize a responder, interpret extensions, compare timestamps to a clock, apply revocation policy, or provide HTTP/TLS.

Run the package tests from the Rust workspace:

```sh
cargo test -p x509-ocsp-single-response -- --nocapture
```
