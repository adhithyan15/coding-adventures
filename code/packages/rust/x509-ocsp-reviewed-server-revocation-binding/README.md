# x509-ocsp-reviewed-server-revocation-binding

`x509-ocsp-reviewed-server-revocation-binding` closes one exact-object gap
between the repository-owned OCSP revocation path and the strongest existing
constructed reviewed-server path evidence. It consumes:

- `AuthenticatedX509OcspRsaSha256TargetRevocation`, retaining the exact target,
  issuer, delegated responder, signed single-status response, freshness
  interval, observation time, and both target/responder issuer links; and
- `ValidatedX509RsaSha256ConstructedReviewedIntermediateServerNameConstraintsPolicy`,
  retaining exact target-server policy, constructed two-link RSA SHA-256 path,
  path-length state, selected Mozilla-reviewed root, selected-anchor DNS
  policy, and supported intermediate DNS-constraint policy.

Binding succeeds only when both inputs retain the same exact decoded target
and intermediate issuer objects and both exact path certificates are valid at
the OCSP proof's retained observation time. Byte-equal independent decodes and
cross-time validity evidence fail closed. The non-copyable result retains both
complete inputs plus fresh exact-time target and issuer validity tokens.
Failures expose only stable mismatch or target/issuer validity categories and
retain no certificate, response, key, signature, status, hostname, or time
payloads.

## Boundary

This crate performs evidence composition and exact-time validity checks only.
It does not perform new cryptography, search for or generalize certificate
paths, implement complete
RFC 5280 policy, validate a responder path to trust, check responder-certificate
or intermediate revocation, interpret unsupported OCSP extensions, acquire or
cache OCSP data, read a clock, implement TLS or HTTP, authorize OAuth
transport, open a network connection, or access a credential. The reviewed
Mozilla root snapshot excludes OS, enterprise, user-installed, and local roots.
`webpki-roots` remains reviewed public data only, not a TLS or validation
implementation. Concrete OAuth HTTPS remains unavailable.

## Public integration-fixture provenance

The checked-in integration fixture is static public evidence captured once on
2026-10-25. It is not fetched by tests and is not a production dependency.

One credential-free OpenSSL 3.6.3 probe used provider `www.apple.com`, SNI
`www.apple.com`, forced TLS 1.2, status request, certificate display, and
handshake-message tracing:

```text
openssl s_client -tls1_2 -connect www.apple.com:443 \
  -servername www.apple.com -status -showcerts -msg
```

The leaf and Apple intermediate are the exact certificates from that one
transcript. The complete stapled `OCSPResponse` is the exact 1,459-byte payload
from that transcript's TLS 1.2 `CertificateStatus` message after removing only
the handshake/status framing. It contains one delegated responder certificate
and one `good` SHA-1 `CertID` status signed with RSA PKCS #1 v1.5 SHA-256. The
response was produced at `2026-10-25T02:29:26Z`, with freshness through
`2026-10-25T18:29:25Z`; tests use explicit observation time
`2026-10-25T12:00:00Z` and never read a clock.

The DigiCert Global Root G2 DER came from DigiCert's official certificate
endpoint:

```text
https://cacerts.digicert.com/DigiCertGlobalRootG2.crt
```

OpenSSL independently verified the leaf for `sslserver`, the embedded
delegated responder for `ocsphelper`, and the response signature and target
status against the same Apple intermediate and DigiCert root before capture.
The repository tests then exercise only repository-owned parsing, hashing,
signature verification, certificate policy, path construction, reviewed-root
selection, name policy, revocation policy, and exact-object binding. System
OpenSSL was fixture-acquisition and independent-check tooling only; it supplies
no production HTTPS, certificate, OCSP, or trust authority.

- Apple leaf DER SHA-256:
  `cd563f5716c254cc1a574cb9f101a923ca50b2822c94c7c85575fff4b809e760`
- Apple intermediate DER SHA-256:
  `340ca5ba402d140b65a2c976e7ae8128a1505c29d190e0e034f59ccae7a92bc2`
- embedded delegated responder DER SHA-256:
  `b3b8febf1451170c65a5211252b74b1a972d09cfabfd5d3484de72aa0dd7110d`
- DigiCert Global Root G2 DER SHA-256:
  `cb3ccbb76031e5e0138f8dd39a23f9de47ffc35e43c1144cea27d46a5ab1cb5f`
- complete stapled OCSP response DER SHA-256:
  `757b42d0666a438b039a6c0e7ab646db5c70913c4e52dd2aceb645763d16ae79`

The acquisition touched only public provider endpoints, used no credential,
performed no signing or edit, and produced no remote mutation. The exact
temporary capture directory was moved to Trash after fixture verification.

## Validation

```bash
bash BUILD
cargo clippy -p x509-ocsp-reviewed-server-revocation-binding --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-ocsp-reviewed-server-revocation-binding --no-deps
```
