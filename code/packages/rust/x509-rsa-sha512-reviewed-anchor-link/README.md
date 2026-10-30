# x509-rsa-sha512-reviewed-anchor-link

Pure evidence binding for one exact RSA SHA-512 issuer-policy link and one
reviewed Mozilla root selection.

The crate consumes:

- `VerifiedX509RsaSha512IssuerPolicyLink`, which already retains the exact
  child, issuer, issuing-CA role policy, data-driven RSA algorithm/key policy,
  and fixed RSA PKCS #1 v1.5 SHA-512 signature verification; and
- `SelectedReviewedWebPkiRootAnchor`, which retains the exact candidate whose
  subject and SPKI identify one unique entry in the reviewed root snapshot.

Binding succeeds only when both inputs retain the same decoded issuer object.
Byte-equal independently decoded certificates are deliberately rejected.

This composition does not apply anchor constraints, validate target role,
purpose, validity, or identity, construct or validate a multi-hop path, check
revocation, perform TLS or HTTP, authorize OAuth transport, or open a network
connection. It does not make concrete HTTPS or live-provider use ready.

## Validation

```bash
bash BUILD
cargo clippy -p x509-rsa-sha512-reviewed-anchor-link --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-rsa-sha512-reviewed-anchor-link --no-deps
```
