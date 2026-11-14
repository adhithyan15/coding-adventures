# x509-rsa-reviewed-anchor-self-signature-binding

Bind routed repository-owned RSA self-signature evidence to one exact reviewed
Web PKI root.

The repository's data-driven signature router can retain proof from its fixed
RSA PKCS #1 v1.5 SHA-256, SHA-384, or SHA-512 verifier. Reviewed-root selection
separately retains one decoded certificate's exact subject-and-SPKI membership
in the pinned Mozilla snapshot. This crate consumes both pieces of evidence and
requires three identities to be the same decoded object:

1. the certificate whose signature was verified;
2. the certificate that supplied the verified RSA public key policy; and
3. the certificate selected from the reviewed snapshot.

The result is non-copyable and keeps both complete inputs. An independently
decoded certificate with identical DER cannot substitute for either side of
the signature proof.

## Security boundary

This package performs no parsing, algorithm selection, RSA operation, root
lookup, credential access, signing, audit, trace, I/O, or external effect. It
does not decide whether a trust anchor must be self-signed, grant trust from a
self-signature, prove CA role, apply time or constraints, validate a path,
check revocation, or authorize TLS, HTTP, OAuth, or network transport.

```text
routed signature child ─────────────┐
routed signature issuer-key policy ├── exact decoded certificate object
reviewed snapshot selection ───────┘
```

Callers first produce `VerifiedX509RsaPolicySignature` using their explicit
profile-index routing table and select the same certificate with
`select_reviewed_webpki_root_anchor`. They can then call
`bind_x509_rsa_reviewed_anchor_self_signature`. A child or issuer object
mismatch returns a stable payload-free error.

## Dependencies

- x509-rsa-signature-routing
- x509-webpki-root-anchor-selection

## Development

```bash
# Run tests
bash BUILD
```
