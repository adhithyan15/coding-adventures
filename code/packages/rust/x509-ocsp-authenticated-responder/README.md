# x509-ocsp-authenticated-responder

`x509-ocsp-authenticated-responder` composes repository-owned OCSP boundaries
for one exact delegated responder candidate. It consumes an exact
`ResponderID`-to-certificate binding and complete responder-certificate policy,
rejects evidence from distinct decoded certificate objects, and validates the
RSA key from that same certificate under caller-owned strength limits.

The data-driven entry point binds that exact response/key pair to caller-owned
algorithm profiles and routes the selected profile to a repository-owned RSA
PKCS #1 v1.5 SHA-256, SHA-384, or SHA-512 verifier. A fixed SHA-256 entry point
remains for compatibility with current callers. Both non-copyable results retain
the identifier binding, certificate policy, and exact signature proof. Errors
expose only stable categories and retain no response, certificate, key,
signature, profile, or route payload.

This package authenticates only a policy-approved delegated responder candidate
to one response. It does not prove that the target certificate's issuer signed
or authorized the responder certificate, construct or validate a responder
path, establish trust, check responder-certificate revocation, bind a `CertID`
or revocation decision, fetch or cache data, read a clock, perform TLS or HTTP,
authorize OAuth transport, or open a network connection. It performs only pure
public-data policy and verification work: no credential access, edit, signing,
storage, clock acquisition, audit or trace emission, I/O, provider action, or
other external effect. Concrete HTTPS and live-provider readiness remain
unavailable.

## Validation

```bash
bash BUILD
cargo clippy -p x509-ocsp-authenticated-responder --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-ocsp-authenticated-responder --no-deps
```
