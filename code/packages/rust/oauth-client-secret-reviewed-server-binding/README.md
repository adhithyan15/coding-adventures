# oauth-client-secret-reviewed-server-binding

This crate joins one complete audited OAuth client-secret HTTP/1 request to one
complete HTTPS destination, reviewed RSA server path, and reviewed-anchor
self-signature proof. It accepts only the repository-owned sealed evidence
types and requires both retained destinations to have:

- the same exact decoded certificate object;
- ASCII-case-insensitively equal SNI DNS names; and
- ASCII-case-insensitively equal Host DNS names with the same optional port.

The separate SNI and Host comparisons prevent substituting another valid SAN
name from the same certificate. A byte-equal independent certificate decode
also fails closed. Success moves both complete non-copyable inputs into one
redacted result.

The composition never asks the request for its credential-bearing wire bytes,
so it introduces no credential access or audit event. Every later byte borrow
still passes through the request release's provider-and-trace audit gate.

This is not a TLS or HTTP client. It performs no certificate parsing, new
validation, cryptography, revocation check, credential edit, signing, I/O, or
external effect. It does not complete RFC 5280 Name or path validation and
does not grant HTTPS, OAuth, network, or live-provider readiness.

## Validation

```bash
bash BUILD
cargo clippy -p oauth-client-secret-reviewed-server-binding --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p oauth-client-secret-reviewed-server-binding --no-deps
```
