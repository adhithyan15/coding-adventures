# x509-validity

`x509-validity` is the allocation-free RFC 5280 `Validity` layer above the
repository-owned `der-asn1` and `x509-time` primitives. It accepts an already
framed ASN.1 element plus the caller's shared decoder, requires exactly the
ordered `notBefore` and `notAfter` time fields, rejects inverted endpoints, and
classifies an explicitly supplied validated time with inclusive bounds.

## Security boundary

The package allocates nothing from wire data, shares the caller's DER depth and
element budgets, and retains no input in errors. It uses no ambient clock,
timezone database, epoch conversion, platform calendar, unsafe code, or
external dependency.

Successful decoding does not establish trust or current validity. This crate
does not parse a complete certificate, validate paths, verify signatures, load
roots, process revocation, match server identities, perform TLS, or open a
network connection.

## Verification

```bash
bash BUILD
cargo clippy -p x509-validity --all-targets -- -D warnings
cargo doc -p x509-validity --no-deps
```
