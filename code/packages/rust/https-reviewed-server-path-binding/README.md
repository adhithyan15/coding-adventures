# https-reviewed-server-path-binding

Repository-owned proof that an HTTPS DNS destination and the strongest
accumulated reviewed RSA server-path policy retain the same exact decoded leaf
certificate object.

The destination input already retains the leaf certificate, certificate SAN
match, TLS SNI, and HTTP/1 Host authority. The server-path input retains the
complete server policy, reviewed root selection, data-driven RSA signature
routing, path-length state, and the currently supported target and subordinate
DNS/IP NameConstraints policy. The binder derives the target only through that
retained path evidence and compares decoded-object identity. A byte-equal
independent decode is rejected.

A stronger entry point consumes the complete reviewed server-path and anchor
self-signature composition instead of the path alone. It derives the same path
target through that non-copyable wrapper, performs the exact leaf-object check,
and retains the complete destination, path, reviewed-root selection, and
repository-owned RSA anchor self-signature proof. The original path-only entry
point remains available when callers intentionally need the weaker boundary.

The strongest current entry point consumes the exact direct no-check complete
fixed-path revocation policy. It derives the server leaf only through that
completion's sealed repository-owned target projection, rejects a byte-equal
independent decode, and retains both the complete destination proof and the
complete revocation proof. The surface accepts no detached certificate,
callback-shaped target claim, path, status result, or trust assertion. Other
complete fixed-path revocation branches remain separate composition slices.

The non-copyable result retains both complete inputs. Stable errors and Debug
output expose no certificate, hostname, authority, path, key, signature,
algorithm, constraint, or root payloads.

This package performs no certificate parsing or new validation, credential
access, signing, storage, clock acquisition, provider action, audit or trace
emission, I/O, network access, or other external effect. It does not complete
Name or path validation, independently validate an anchor self-signature,
acquire or refresh revocation data, generalize revocation or path validation,
provide TLS or HTTP, authorize OAuth transport, or grant network authority or
live-provider readiness. Concrete HTTPS remains unavailable.

## Dependencies

- https-destination-name-binding
- x509-ocsp-reviewed-server-revocation-binding
- x509-rsa-constructed-path-signature-routing
- x509-rsa-reviewed-server-path-anchor-self-signature-binding

## Development

```bash
bash BUILD
cargo clippy -p https-reviewed-server-path-binding --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p https-reviewed-server-path-binding --no-deps
```
