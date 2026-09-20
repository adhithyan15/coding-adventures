# x509-certificate-version

`x509-certificate-version` is the allocation-free RFC 5280 certificate-version
layer above the repository-owned `der-asn1` primitive. It decodes the optional
`[0] EXPLICIT Version` wrapper when present, accepts only `v2` or `v3` there,
and exposes `V1` as the default for an omitted field.

## Security boundary

The package allocates nothing from wire data, shares the caller's DER depth and
element budgets, retains no input in errors, uses no unsafe code, and has no
external dependency.

Successful decoding does not enforce which later `TBSCertificate` fields are
legal for that version. This crate does not parse a complete certificate,
validate paths, verify signatures, load roots, process revocation, perform TLS,
or open a network connection.

## Verification

```bash
bash BUILD
cargo clippy -p x509-certificate-version --all-targets -- -D warnings
cargo doc -p x509-certificate-version --no-deps
```
