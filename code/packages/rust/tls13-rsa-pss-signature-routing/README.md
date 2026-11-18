# tls13-rsa-pss-signature-routing

Policy-bound TLS 1.3 RSA-PSS signature routing.

This package accepts one structurally decoded TLS 1.3 `CertificateVerify`
message, an already policy-authorized RSA-PSS RSAE scheme, a validated
`rsaEncryption` public key, a typed caller-computed digest, fixed-profile
verification limits, and caller-owned scratch memory. It rejects cross-profile
wire IDs, digests, and limits before RSA work, then routes to the matching
repository-owned SHA-256, SHA-384, or SHA-512 RSASSA-PSS verifier.

Successful verification returns non-copyable evidence retaining the exact
authorization, message, key, digest, and selected fixed-verifier evidence.
Errors and debug output omit signature, digest, key, and caller lengths.

```text
authorized scheme + decoded message + validated key + typed digest
    + caller scratch + matching typed limits
    -> verified fixed-profile evidence
```

The package deliberately does not construct or hash the TLS 1.3
`CertificateVerify` signed input and does not prove transcript provenance,
certificate parsing, path construction, trust, hostname validation,
revocation, handshake sequencing, record protection, TLS/HTTP composition, or
OAuth transport readiness.

## Development

```bash
# Run the package validation entrypoint
bash BUILD
```
