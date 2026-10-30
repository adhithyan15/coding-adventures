# x509-rsa-sha512-reviewed-server-link

Pure evidence binding for one exact target-server policy result and either one
authenticated RSA SHA-512 issuer link terminating at a reviewed Mozilla root
or one authenticated target-to-intermediate-to-reviewed-root path.

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

`bind_x509_rsa_sha512_reviewed_intermediate_server_path` performs the analogous
identity gate for an existing two-link proof. It requires the server-policy
certificate to be the exact target object authenticated by the proof's first
link, then retains complete target policy, intermediate policy, both
authenticated links, and reviewed-root selection together. It does not advance
path-length state, apply anchor or intermediate constraints, construct or
search paths, or generalize certificate-path validation.

`validate_x509_rsa_sha512_reviewed_server_anchor_policy` then consumes that
complete proof and derives both inputs to its DNS-policy decision from it:

- the selected reviewed snapshot entry's optional out-of-band
  `NameConstraints` bytes; and
- the exact retained target certificate's Subject Alternative Name extension.

Separate caller-owned ASN.1 limits bound both decoders. An unconstrained anchor
succeeds without manufacturing DNS evidence. When constraints are present,
every target `dNSName` must satisfy the repository-owned supported permitted
DNS subtree policy. Malformed or unsupported constraints, a missing or
malformed target SAN, and a name outside the permitted subtrees fail with
payload-free categories. The non-copyable success retains the complete input
proof beside optional constrained-name evidence.

This composition performs no new signature work or root lookup. Its bounded
parsing is limited to the direct-link policy's exact reviewed-anchor constraint
bytes and exact target SAN retained by the consumed proof. The two-link binding
performs no parsing. Neither API constructs or validates a path, applies
intermediate or IP constraints, checks revocation or hostnames, performs TLS or
HTTP, authorizes OAuth transport, or opens a network connection. It does not
make concrete HTTPS or live-provider use ready.

## Validation

```bash
bash BUILD
cargo clippy -p x509-rsa-sha512-reviewed-server-link --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-rsa-sha512-reviewed-server-link --no-deps
```
