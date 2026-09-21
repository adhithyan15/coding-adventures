# x509-extended-key-usage

`x509-extended-key-usage` is the allocation-free RFC 5280 Extended Key Usage
semantic layer above the repository-owned `x509-extension` and `der-asn1`
primitives. It recognizes only OID `2.5.29.37` and decodes a non-empty, bounded
sequence of arbitrary canonical purpose OIDs in wire order.

## Security boundary

The package retains at most 64 borrowed purpose identifiers, allocates nothing
from wire data, shares the caller's depth and element budgets, and retains no
input in errors. It uses no unsafe code or external dependency.

Successful decoding does not authorize any key purpose or assign a certificate
role. This crate does not enforce outer criticality, interpret known or unknown
purpose OIDs, combine Extended Key Usage with Key Usage, parse a complete
certificate, construct or validate paths, verify signatures, load roots,
process revocation, match server identities, perform TLS, or open a network
connection.

## Verification

```bash
bash BUILD
cargo clippy -p x509-extended-key-usage --all-targets -- -D warnings
cargo doc -p x509-extended-key-usage --no-deps
```
