# x509-extensions

`x509-extensions` is the allocation-free bounded RFC 5280 `Extensions` layer
above the repository-owned `x509-extension` primitive. It decodes a non-empty
sequence of at most 64 generic extensions and rejects duplicate extension
identifiers while preserving wire order.

## Security boundary

The package shares the caller's DER depth, element, and OID-arc limits, retains
no input in errors, allocates nothing from wire data, and denies unsafe code.
It has no ambient authority or external dependency.

Successful decoding does not recognize or process any extension. A later
data-driven policy must recognize each OID, reject unsupported critical
extensions, and decode the encapsulated value. This crate performs no
certificate or path validation, cryptography, hostname verification,
trust-root loading, TLS, or network access.

## Verification

```bash
bash BUILD
cargo clippy -p x509-extensions --all-targets -- -D warnings
cargo doc -p x509-extensions --no-deps
```
