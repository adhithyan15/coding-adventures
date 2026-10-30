# x509-rsa-sha512-reviewed-server-link

Pure evidence binding for one exact target-server policy result and one
authenticated RSA SHA-512 issuer link terminating at a reviewed Mozilla root.

The crate consumes:

- `ValidatedX509ServerCertificatePolicy`, which retains certificate-wide
  critical-extension and end-entity role policy, caller-selected algorithm
  policy, SAN-only identity, explicit-time validity, and exact Extended Key
  Usage purpose for one decoded target; and
- `BoundX509RsaSha512ReviewedAnchorLink`, which retains data-driven RSA
  algorithm/key policy, fixed repository-owned RSA PKCS #1 v1.5 SHA-512
  signature verification, and reviewed-root selection for one exact directed
  target-to-issuer link.

Binding succeeds only when both inputs retain the same decoded target object.
Byte-equal independently decoded certificates are deliberately rejected.

This composition performs no new parsing or signature work. It does not apply
reviewed-anchor constraints, construct or validate a multi-hop path, check
revocation, perform TLS or HTTP, authorize OAuth transport, or open a network
connection. It does not make concrete HTTPS or live-provider use ready.

## Validation

```bash
bash BUILD
cargo clippy -p x509-rsa-sha512-reviewed-server-link --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-rsa-sha512-reviewed-server-link --no-deps
```
