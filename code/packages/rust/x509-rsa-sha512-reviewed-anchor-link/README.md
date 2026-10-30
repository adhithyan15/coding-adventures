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

`bind_x509_rsa_sha512_intermediate_path_to_reviewed_anchor` joins one complete-
intermediate-policy-bound target-to-intermediate link to one policy-bound
intermediate-to-reviewed-root link. It requires the first link's candidate
issuer and the reviewed parent link's authenticated child to retain the same
exact decoded intermediate object; byte-equal independent decodes fail closed.
The returned non-copyable evidence keeps complete intermediate policy, both
authenticated links, and the exact reviewed-root selection together.

`bind_x509_rsa_sha512_authenticated_intermediate_path_to_reviewed_anchor`
provides the stronger additive boundary. It consumes the existing
path-length-advanced authenticated intermediate proof instead of the narrower
link proof, applies the same exact-object join to the reviewed parent link, and
retains the advanced state with both authenticated links, complete
intermediate policy, and reviewed-root selection. The earlier API remains
available for callers that have not yet advanced path length.

This is not complete certificate-path validation. It does not apply anchor
name constraints, derive an initial path-length state, validate target role,
purpose, time, hostname, or revocation, search a path, perform TLS or HTTP, or
authorize OAuth network access. The pinned `webpki-roots` snapshot remains
reviewed public data only; no external TLS, X.509, or cryptographic
implementation is linked. It does not make concrete HTTPS or live-provider use
ready.

## Validation

```bash
bash BUILD
cargo clippy -p x509-rsa-sha512-reviewed-anchor-link --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-rsa-sha512-reviewed-anchor-link --no-deps
```
