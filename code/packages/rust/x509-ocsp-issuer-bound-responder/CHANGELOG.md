# Changelog

## Unreleased

- Add a non-copyable issuer-binding result for the data-driven authenticated
  responder proof while preserving the existing fixed SHA-256 API.
- Retain caller-routed SHA-256, SHA-384, or SHA-512 response-signature evidence
  while keeping the responder-certificate issuer link explicitly RSA SHA-256.
- Reject byte-equal independently decoded responder substitutions and preserve
  payload-free failures without adding credentials, I/O, or external effects.

## 0.1.0 — 2026-10-25

- Add non-copyable evidence binding one exact authenticated OCSP responder to
  one exact candidate issuer.
- Reject cross-certificate evidence mixing before returning issuer-binding
  evidence.
- Retain the existing responder authentication and RSA SHA-256 issuer-link
  proofs without granting target-certificate, path, trust, revocation, TLS,
  HTTP, OAuth transport, or network authority.
