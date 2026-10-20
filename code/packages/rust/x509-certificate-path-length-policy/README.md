# x509-certificate-path-length-policy

A bounded repository-owned RFC 5280 path-length state transition for
caller-ordered intermediate CA certificates.

The caller initializes an explicit maximum and advances from the trust-anchor
side toward the target. Each step requires existing issuing-CA role evidence.
A non-self-issued intermediate consumes one unit; a self-issued intermediate
does not. The intermediate's decoded `pathLenConstraint`, when present, can
only tighten the remaining maximum.

The caller remains responsible for deciding whether a certificate is
self-issued and for supplying the correct path order. The target certificate
and trust anchor are not passed through this transition. This crate does not
construct, link, authenticate, or trust a path; process trust-anchor or name
constraints; verify signatures, names, validity, purpose, or revocation; or
provide TLS, HTTP, OAuth transport, or network authority.

## Validation

```bash
bash BUILD
cargo clippy -p x509-certificate-path-length-policy --all-targets -- -D warnings
cargo doc -p x509-certificate-path-length-policy --no-deps
```
