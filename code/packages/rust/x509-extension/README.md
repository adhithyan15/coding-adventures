# x509-extension

`x509-extension` is the allocation-free generic RFC 5280 `Extension` layer
above the repository-owned `der-asn1` primitive. It borrows one validated
extension OID and one opaque OCTET STRING while enforcing DER omission of the
default `critical = FALSE` field.

## Security boundary

The package shares the caller's DER depth, element, and OID-arc limits, retains
no input in errors, allocates nothing from wire data, and denies unsafe code.
It has no ambient authority or external dependency.

Successful decoding does not recognize or process an extension. A later
data-driven policy must recognize the OID, handle unknown critical extensions,
and decode the encapsulated value. This crate performs no certificate or path
validation, cryptography, hostname verification, trust-root loading, TLS, or
network access.

## Verification

```bash
bash BUILD
cargo clippy -p x509-extension --all-targets -- -D warnings
cargo doc -p x509-extension --no-deps
```
