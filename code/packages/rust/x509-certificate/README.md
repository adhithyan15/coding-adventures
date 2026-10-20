# x509-certificate

`x509-certificate` is the allocation-free structural RFC 5280 certificate
composition above the repository-owned DER and X.509 field primitives. It
decodes the complete `Certificate` and `TBSCertificate` shapes, preserves the
exact signed bytes, enforces optional-field ordering and version constraints,
requires a non-empty issuer Name, and requires the inner and outer signature
algorithm encodings to match.

## Security boundary

The package uses one caller-selected DER depth, element, framing, and OID
budget. Decoded values borrow the caller's input, errors retain only a category
and byte offset, and unsafe code is denied. It has no ambient authority or
external dependency.

Successful structural decoding does not recognize or enable an algorithm,
verify the certificate signature, authorize key use, normalize names, validate
a hostname or path, process extension policy, check revocation, select trust
roots, perform TLS, read a clock, or open a network connection. Those remain
separate policy and cryptographic boundaries.

## Verification

```bash
bash BUILD
cargo clippy -p x509-certificate --all-targets -- -D warnings
cargo doc -p x509-certificate --no-deps
```
