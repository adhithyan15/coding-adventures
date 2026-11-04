# Changelog

## Unreleased

- Add an additive data-driven responder-authentication entry point that binds
  the exact matched response and responder-certificate RSA key to caller-owned
  algorithm profiles and SHA-256, SHA-384, or SHA-512 verifier routes.
- Retain the fixed SHA-256 API for compatibility, preserve non-copyable exact
  evidence, and keep all new policy and routing failures payload-free.

## 0.1.0 — 2026-10-25

- Add non-copyable exact delegated-responder-candidate authentication evidence.
- Reject cross-certificate evidence mixing before public-key or signature work.
- Derive a caller-policy-validated RSA key from the exact responder certificate
  and verify the exact matched response with repository-owned PKCS #1 v1.5
  SHA-256 primitives.
- Keep failures redacted and grant no issuer delegation, path, trust,
  revocation, TLS, HTTP, OAuth transport, or network authority.
