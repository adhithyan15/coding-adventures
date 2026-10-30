# x509-rsa-sha384-reviewed-anchor-link

Exact evidence binding between one repository-owned RSA SHA-384 issuer-policy
link and one selection from the pinned reviewed Mozilla public-Web-PKI root
snapshot.

`bind_x509_rsa_sha384_issuer_link_to_reviewed_anchor` succeeds only when both
non-copyable inputs retain the same decoded issuer certificate object. Encoded
byte equality is insufficient. The result keeps the complete directed Name,
certificate-wide critical-extension and issuing-CA role, data-driven RSA
algorithm/key, fixed SHA-384 signature, and reviewed subject-plus-SPKI
membership evidence together.

This is not complete certificate-path validation. It does not apply anchor
name constraints, validate target role, purpose, time, hostname, path length,
or revocation, search a path, perform TLS or HTTP, or authorize OAuth network
access. The pinned `webpki-roots` snapshot remains reviewed public data only;
no external TLS, X.509, or cryptographic implementation is linked.

The success test uses Microsoft's public RSA Root Certificate Authority 2017
solely as an interoperability vector. Its SHA-256 fingerprint is
`c741f70f4b2a8d88bf2e71c14122ef53ef10eba0cfa5e64cfa20f418853073e0`.

## Development

```bash
cargo test -p x509-rsa-sha384-reviewed-anchor-link --all-targets
cargo clippy -p x509-rsa-sha384-reviewed-anchor-link --all-targets -- -D warnings
```
