# x509-serial-number

`x509-serial-number` is the allocation-free RFC 5280 certificate serial-number
layer above the repository-owned `der-asn1` primitive. It accepts an already
framed ASN.1 element, requires a canonical positive nonzero INTEGER with no
more than 20 content octets, and retains only borrowed canonical and normalized
magnitude views.

## Security boundary

The package allocates nothing from wire data, inherits the caller's DER limits,
retains no input in errors, uses no unsafe code, and has no external dependency.

Successful decoding does not prove issuer uniqueness or parse a complete
certificate. This crate does not issue serial numbers, compare issuer names,
validate paths, verify signatures, load roots, process revocation, perform TLS,
or open a network connection.

## Verification

```bash
bash BUILD
cargo clippy -p x509-serial-number --all-targets -- -D warnings
cargo doc -p x509-serial-number --no-deps
```
