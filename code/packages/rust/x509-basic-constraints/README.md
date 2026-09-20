# x509-basic-constraints

`x509-basic-constraints` is the allocation-free RFC 5280 Basic Constraints
semantic layer above the repository-owned `x509-extension` and `der-asn1`
primitives. It recognizes only OID `2.5.29.19`, decodes the exact encapsulated
sequence, rejects an encoded default false CA flag, and accepts only
non-negative path lengths when CA is true.

## Security boundary

The package borrows canonical path-length bytes instead of narrowing them to a
machine integer, allocates nothing from wire data, shares the caller's total
element budget, and retains no input in errors. It uses no unsafe code or
external dependency.

Successful decoding does not grant CA authority or establish trust. This crate
does not enforce outer criticality or Key Usage policy, parse a complete
certificate, construct or validate paths, verify signatures, load roots,
process revocation, match server identities, perform TLS, or open a network
connection.

## Verification

```bash
bash BUILD
cargo clippy -p x509-basic-constraints --all-targets -- -D warnings
cargo doc -p x509-basic-constraints --no-deps
```
