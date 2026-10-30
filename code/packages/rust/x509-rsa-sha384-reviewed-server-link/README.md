# x509-rsa-sha384-reviewed-server-link

Pure evidence binding for one exact target-server policy result and one
authenticated RSA SHA-384 issuer link terminating at a reviewed Mozilla root.

The crate consumes:

- `ValidatedX509ServerCertificatePolicy`, which retains certificate-wide
  critical-extension and end-entity role policy, caller-selected algorithm
  policy, SAN-only identity, explicit-time validity, and exact Extended Key
  Usage purpose for one decoded target; and
- `BoundX509RsaSha384ReviewedAnchorLink`, which retains data-driven RSA
  algorithm/key policy, fixed repository-owned RSA PKCS #1 v1.5 SHA-384
  signature verification, and reviewed-root selection for one exact directed
  target-to-issuer link.

Binding succeeds only when both inputs retain the same decoded target object.
Byte-equal independently decoded certificates are deliberately rejected.

`validate_x509_rsa_sha384_reviewed_server_anchor_policy` then derives the
selected reviewed anchor's optional NameConstraints bytes and the exact
target certificate only from that consumed binding. Separate caller-owned
ASN.1 limits bound the reviewed-root constraint decoder and target Subject
Alternative Name decoder. A constrained anchor requires every target dNSName
to satisfy the supported permitted subtree set; malformed inputs, unsupported
constraint forms, and missing SAN fail with payload-free categories. The
non-copyable result retains the complete binding and optional constrained-name
evidence.

This composition performs no new signature work. It does not apply IP or
intermediate constraints, construct or validate a multi-hop path, verify a
hostname, check revocation, perform TLS or HTTP, authorize OAuth transport, or
open a network connection. It does not make concrete HTTPS or live-provider
use ready.

## Validation

```bash
bash BUILD
cargo clippy -p x509-rsa-sha384-reviewed-server-link --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-rsa-sha384-reviewed-server-link --no-deps
```
