# x509-ocsp-basic-response

Allocation-free structural decoding for one RFC 6960
`BasicOCSPResponse` value:

- one structural `ResponseData` value whose exact DER encoding is the signature
  payload;
- one generic signature `AlgorithmIdentifier`;
- one canonical borrowed signature BIT STRING;
- an optional `[0] EXPLICIT` certificate sequence with a conservative fixed
  ceiling of one complete decoded certificate; and
- exact complete response bytes plus shared DER depth, element, length,
  OID-arc, extension-count, response-count, and certificate-count limits.

The one-certificate ceiling bounds the composed value to roughly 105 KiB on
the supported target: a complete allocation-free certificate already embeds
fixed-capacity Name and Extensions storage. A second certificate fails closed
on the default test stack instead of allocating or silently resetting parser
budgets. An explicitly present empty certificate sequence remains structurally
valid, although RFC 6960 says the optional field should be absent when no
certificates are included.

## Security boundary

This crate parses syntax only. It does not recognize or enable the signature
algorithm, verify the signature, choose or authorize a responder certificate,
match `ResponderID`, construct or validate a path, apply certificate role or
critical-extension policy, compare times to a clock, decide revocation, or
provide HTTP, TLS, OAuth transport, or network access.

Run the package tests from the Rust workspace:

```sh
cargo test -p x509-ocsp-basic-response -- --nocapture
```
