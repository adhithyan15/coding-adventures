# Changelog

## 0.1.0 — 2026-10-25

- Add non-copyable exact-certificate OCSP responder policy evidence.
- Bind certificate-wide end-entity role and data-driven algorithm policy before
  requiring compatible present Key Usage and deriving explicit-time validity
  and fixed OCSP-signing EKU results.
- Reject byte-equal evidence from independently decoded certificate objects.
- Keep failures redacted and grant no response-signature, issuer, path, trust,
  revocation, TLS, HTTP, OAuth transport, or network authority.
