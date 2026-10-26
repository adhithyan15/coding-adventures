# x509-ocsp-authenticated-responder

`x509-ocsp-authenticated-responder` composes three repository-owned OCSP
boundaries for one exact delegated responder candidate. It consumes an exact
`ResponderID`-to-certificate binding and complete responder-certificate policy,
rejects evidence from distinct decoded certificate objects, validates the RSA
key from that same certificate under caller-owned strength limits, and verifies
the matched response's preserved `ResponseData` signature with repository-owned
RSA PKCS #1 v1.5 SHA-256.

The non-copyable result retains the identifier binding, certificate policy, and
signature proof. Errors expose only stable categories and retain no response,
certificate, key, or signature bytes.

This package authenticates only a policy-approved delegated responder candidate
to one response. It does not prove that the target certificate's issuer signed
or authorized the responder certificate, construct or validate a responder
path, establish trust, check responder-certificate revocation, bind a `CertID`
or revocation decision, fetch or cache data, read a clock, perform TLS or HTTP,
authorize OAuth transport, or open a network connection.

## Validation

```bash
bash BUILD
cargo clippy -p x509-ocsp-authenticated-responder --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-ocsp-authenticated-responder --no-deps
```
