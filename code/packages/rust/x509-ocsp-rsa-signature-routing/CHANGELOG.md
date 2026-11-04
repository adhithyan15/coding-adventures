# Changelog

## Unreleased

- Route caller-selected profiles through the existing repository-owned OCSP
  RSA PKCS #1 v1.5 SHA-384 verifier and retain exact policy/signature evidence.
- Map SHA-384 verifier failures into the existing payload-free routing error
  surface and cover successful and mismatched routes with public fixture data.

## 0.1.0

- Add caller-owned profile-index routing for the fixed repository-owned OCSP
  RSA PKCS #1 v1.5 SHA-256 and SHA-512 signature verifiers.
- Retain the consumed response/key algorithm-policy evidence alongside the
  exact fixed-verifier result.
