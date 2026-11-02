# x509-intermediate-certificate-policy

`x509-intermediate-certificate-policy` composes the already implemented X.509
policy prerequisites for one exact intermediate certificate. It consumes:

- certificate-wide critical-extension and issuing-CA role evidence;
- certificate-bound, caller-selected algorithm-profile evidence; and
- caller-policy self-issued classification.

The composition rejects evidence unless every input retains the same decoded
certificate object and the role is exactly `IssuingCertificateAuthority`. It
then validates that certificate's inclusive validity interval at one explicit
caller-owned `X509Time` observation. The non-copyable success value retains
all consumed inputs plus the validity result, giving later path code one
non-mixable intermediate-certificate policy boundary.

A follow-up consumes that complete boundary and inspects only its exact
certificate for NameConstraints. Absence is retained as `None`; a present
extension must be critical and must decode under a separate caller-owned ASN.1
budget. Success retains borrowed DNS/IP constraints beside the complete
non-copyable intermediate evidence for later path policy.

The caller still owns critical-extension recognition, algorithm profiles, Name
comparison semantics, and trustworthy time acquisition. Decoding does not
evaluate any constraint against a certificate. The crate does not authenticate
adjacent certificate links, advance path-length state, construct or validate a
certification path, apply anchor constraints, select trust, check revocation,
perform TLS or HTTP, authorize OAuth transport, or open a network connection.

## Development

```sh
cargo test -p x509-intermediate-certificate-policy --test x509_intermediate_certificate_policy -- --nocapture
cargo test -p x509-intermediate-certificate-policy --doc
cargo clippy -p x509-intermediate-certificate-policy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-intermediate-certificate-policy --no-deps
```
