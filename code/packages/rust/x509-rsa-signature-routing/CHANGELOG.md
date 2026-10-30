# Changelog

## 0.1.0

- Add caller-owned child-profile routing for the fixed repository-owned X.509
  RSA PKCS #1 v1.5 SHA-256, SHA-384, and SHA-512 signature verifiers.
- Retain the consumed exact child algorithm policy and exact issuer RSA
  key-policy evidence alongside the fixed-verifier result.
- Project the exact retained child and issuer policy evidence through
  verifier-independent borrowed accessors on routed success.
