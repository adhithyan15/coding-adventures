# Changelog

## 0.1.0 — 2026-10-25

- Add non-copyable evidence binding one exact authenticated OCSP responder to
  one exact candidate issuer.
- Reject cross-certificate evidence mixing before returning issuer-binding
  evidence.
- Retain the existing responder authentication and RSA SHA-256 issuer-link
  proofs without granting target-certificate, path, trust, revocation, TLS,
  HTTP, OAuth transport, or network authority.
