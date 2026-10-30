# x509-server-certificate-policy

`x509-server-certificate-policy` composes existing repository-owned X.509
policy gates for one exact decoded target certificate. It consumes non-copyable
certificate-wide end-entity role evidence and certificate-bound algorithm
evidence, rejects cross-object mixing, then derives SAN-only server identity,
inclusive validity, and exact Extended Key Usage purpose results from that same
certificate.

All policy inputs remain explicit: the caller chooses critical-extension
recognition before constructing role evidence, the accepted algorithm profile,
the reference DNS name or IP address, the time observation, the requested EKU
OID, and the ASN.1 budget. Errors expose only stable categories and retain no
certificate bytes, names, times, or purpose identifiers.

An explicit follow-up consumes complete server policy and re-decodes only the
exact Subject Alternative Name extension index retained by its successful
identity result under a fresh caller-owned ASN.1 budget. The non-copyable
result retains the complete server policy beside the decoded DNS/IP collection
for later NameConstraints composition; callers cannot substitute another
certificate or extension.

SAN retention does not itself evaluate NameConstraints. This package does not
authenticate an issuer, select or constrain a trust
anchor, construct or validate a certification path, check revocation, perform
TLS or HTTP, authorize OAuth transport, or open a network connection.

## Validation

```bash
bash BUILD
cargo clippy -p x509-server-certificate-policy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-server-certificate-policy --no-deps
```
