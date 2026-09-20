# x509-name

`x509-name` is the allocation-free RFC 5280 distinguished-name structural
layer above the repository-owned `der-asn1` primitive. It decodes a bounded
RDN sequence, requires every RDN SET to be non-empty and DER ordered, and
retains borrowed attribute OIDs plus exact opaque attribute values.

## Security boundary

The package allocates nothing from wire data, shares the caller's DER depth and
element budgets, caps both RDN and total attribute counts, retains no input in
errors, uses no unsafe code, and has no external dependency.

Successful structural decoding does not normalize DirectoryString values or
establish semantic distinguished-name equality. This crate does not parse a
complete certificate, validate paths, verify signatures, load roots, process
revocation, perform TLS, or open a network connection.

## Verification

```bash
bash BUILD
cargo clippy -p x509-name --all-targets -- -D warnings
cargo doc -p x509-name --no-deps
```
