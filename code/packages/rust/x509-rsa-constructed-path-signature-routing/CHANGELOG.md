# Changelog

## 0.1.0 - 2026-10-30

### Added

- Added exact-object binding between one constructed-path adjacency, child
  algorithm policy, issuer role policy, and issuer RSA algorithm/key policy.
- Added data-driven routing to the repository-owned RSA PKCS #1 v1.5 SHA-256,
  SHA-384, and SHA-512 certificate-signature verifiers.
- Added non-copyable success evidence and payload-free failure categories.

### Security

- Cross-certificate evidence and non-CA issuers fail before RSA work.
- The composition grants no complete path, trust, revocation, TLS, HTTP,
  OAuth transport, or network authority.
