# x509-ocsp-responder-certificate-policy

`x509-ocsp-responder-certificate-policy` composes repository-owned X.509
policy gates for one exact decoded candidate OCSP responder certificate. It
consumes non-copyable certificate-wide role evidence and certificate-bound,
data-driven algorithm evidence, rejects cross-object mixing, requires the
end-entity role, requires a present Key Usage to permit `digitalSignature` or
`contentCommitment`, then derives explicit-time validity and the fixed RFC 6960
`id-kp-OCSPSigning` Extended Key Usage result from that same certificate.

Critical-extension recognition and algorithm profiles remain explicit inputs
to the prerequisite evidence; the time observation is explicit here. Errors
expose only stable categories and retain no certificate bytes, times, or OIDs.

This package does not bind a `ResponderID`, verify or bind an OCSP response
signature, prove issuer or delegated authorization, construct or validate a
certification path, establish trust, check responder-certificate revocation,
fetch or cache revocation data, read a clock, perform TLS or HTTP, authorize
OAuth transport, or open a network connection.

## Validation

```bash
bash BUILD
cargo clippy -p x509-ocsp-responder-certificate-policy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-ocsp-responder-certificate-policy --no-deps
```
