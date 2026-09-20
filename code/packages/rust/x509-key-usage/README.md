# x509-key-usage

`x509-key-usage` is the allocation-free RFC 5280 Key Usage semantic layer
above the repository-owned `x509-extension` and `der-asn1` primitives. It
recognizes only OID `2.5.29.15`, decodes the exact encapsulated BIT STRING,
requires at least one asserted usage, and rejects undefined or non-minimally
encoded named bits.

## Security boundary

The package normalizes only the nine defined RFC usage bits, allocates nothing
from wire data, shares the caller's total element budget, and retains no input
in errors. It uses no unsafe code or external dependency.

Successful decoding grants no certificate authority, end-entity role, or key
operation. This crate does not enforce outer criticality, combine Key Usage
with Basic Constraints or Extended Key Usage, parse a complete certificate,
construct or validate paths, verify signatures, load roots, process
revocation, match server identities, perform TLS, or open a network connection.

## Verification

```bash
bash BUILD
cargo clippy -p x509-key-usage --all-targets -- -D warnings
cargo doc -p x509-key-usage --no-deps
```
