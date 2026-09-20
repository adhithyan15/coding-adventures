# x509-subject-public-key-info

`x509-subject-public-key-info` is the allocation-free generic RFC 5280
`SubjectPublicKeyInfo` layer above the repository-owned DER and
`AlgorithmIdentifier` primitives. It borrows one validated algorithm
identifier and one canonical BIT STRING without interpreting either.

## Security boundary

The package shares the caller's DER depth, element, and OID-arc limits, retains
no input in errors, allocates nothing from wire data, and denies unsafe code.
It has no ambient authority or external dependency.

Successful decoding does not enable an algorithm or validate a key. A later
data-driven policy must recognize the OID, validate its parameters, and parse
the BIT STRING before any key or signature operation. This crate performs no
cryptography, certificate or path validation, trust-root loading, TLS, or
network access.

## Verification

```bash
bash BUILD
cargo clippy -p x509-subject-public-key-info --all-targets -- -D warnings
cargo doc -p x509-subject-public-key-info --no-deps
```
