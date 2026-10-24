# x509-ocsp-cert-id

`x509-ocsp-cert-id` is the allocation-free structural RFC 6960 `CertID` layer
above the repository-owned DER, generic `AlgorithmIdentifier`, and certificate
serial-number primitives. It borrows one exact hash algorithm identifier, the
issuer-name and issuer-key hash OCTET STRING contents, and one validated
positive RFC 5280 serial number.

## Security boundary

The package shares the caller's DER depth, element, length, and OID-arc limits,
retains no identifier bytes in errors, redacts identifier values from `Debug`,
allocates nothing from wire data, and denies unsafe code. It has no ambient
authority or external dependency.

Successful decoding proves only that one value has the RFC 6960 `CertID`
shape. The hash lengths intentionally remain data-driven: a later policy must
recognize the algorithm and validate its parameters and digest size. A later
binding step must compute the issuer hashes and compare the serial number
against the exact certificate and issuer.

This package does not parse an OCSP response, authenticate a response
signature, authorize a responder, bind to a certificate or path, decide
revocation status or freshness, fetch or cache data, read a clock, establish
trust, perform TLS or HTTP, authorize OAuth transport, or access a network.

## Verification

```bash
bash BUILD
cargo clippy -p x509-ocsp-cert-id --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-ocsp-cert-id --no-deps
```
