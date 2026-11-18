# tls13-rsa-pss-certificate-verify

Bound TLS 1.3 RSA-PSS `CertificateVerify` hashing and verification.

This package closes the caller-digest substitution gap between exact TLS 1.3
`CertificateVerify` signed-input framing and repository-owned RSA-PSS routing.
It exposes two deliberately separate stages:

1. Hash one exact framed signed input with the SHA-256, SHA-384, or SHA-512
   primitive fixed by an already policy-authorized RSA-PSS RSAE scheme.
2. Verify one exact structurally decoded `CertificateVerify` message using only
   that retained hash evidence, a validated `rsaEncryption` key, typed limits,
   and caller-owned scratch.

The first stage owns the digest and retains the exact authorization and input.
The second stage returns non-copyable evidence retaining that hash evidence and
the fixed-verifier routing result. Errors and debug output omit input,
transcript-hash, signature, digest, key, limit, and scratch bytes.

```text
authorized scheme + exact framed signed input
    -> retained hash evidence
    + exact decoded signature + validated RSA key + matching limits + scratch
    -> bound CertificateVerify evidence
```

This package does not prove which handshake transcript supplied the framed
input's transcript hash, bind that transcript to a negotiated cipher suite or
ordered handshake state, parse certificates, bind the key to a validated
server path, establish trust, verify a hostname, check time or revocation,
authenticate Finished, sequence a handshake, protect records, compose TLS/HTTP,
perform I/O, authorize OAuth transport, or provide concrete HTTPS or
live-provider readiness.

## Development

```bash
# Run the package validation entrypoint
bash BUILD
```
