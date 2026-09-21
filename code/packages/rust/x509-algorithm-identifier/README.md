# x509-algorithm-identifier

`x509-algorithm-identifier` is the allocation-free generic RFC 5280
`AlgorithmIdentifier` layer above the repository-owned `der-asn1` primitive.
It requires one validated algorithm OID and borrows zero or one canonical DER
parameter element without interpreting it.

## Security boundary

The package shares the caller's DER depth, element, and OID-arc limits, retains
no input in errors, allocates nothing from wire data, and denies unsafe code.
It has no ambient authority or external dependency.

Successful decoding does not enable an algorithm. A later data-driven policy
must recognize the OID and validate the exact parameter contract before any
key or signature operation. This crate performs no cryptography, certificate
or path validation, trust-root loading, TLS, or network access.

## Verification

```bash
bash BUILD
cargo clippy -p x509-algorithm-identifier --all-targets -- -D warnings
cargo doc -p x509-algorithm-identifier --no-deps
```
