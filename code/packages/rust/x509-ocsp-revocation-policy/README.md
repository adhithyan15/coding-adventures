# x509-ocsp-revocation-policy

Exact status and freshness policy for one OCSP `SingleResponse` whose `CertID`
was already bound to exact target and issuer certificate objects.

The crate accepts non-copyable `BoundX509OcspCertId` evidence and one explicit
caller-owned validated time. It derives the status, `thisUpdate`, and
`nextUpdate` only from the exact retained response, rejects every currently
unsupported `singleExtensions` collection, and applies the repository-owned
strict revocation policy. Only `good` inside the ordered half-open interval
`thisUpdate <= observation_time < nextUpdate` succeeds. The non-copyable result
retains the exact CertID binding and observation time, preventing detached
status or freshness values from being attached to another certificate.

## Security boundary

The CertID binding proves identifier equality only. This package does not
authenticate the OCSP response signature, bind or authorize a responder,
interpret outer response extensions, prove the candidate issuer signed the
target, bind the result to a constructed certificate path, fetch or cache OCSP,
read a clock, implement nonce policy, establish trust, perform TLS or HTTP, or
authorize OAuth transport or network effects. It therefore does not make
concrete HTTPS or live-provider use ready.

## Development

```bash
cargo test -p x509-ocsp-revocation-policy --all-targets
cargo clippy -p x509-ocsp-revocation-policy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-ocsp-revocation-policy --no-deps
```
