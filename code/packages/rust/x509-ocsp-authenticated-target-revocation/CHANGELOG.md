# Changelog

## Unreleased

- Add non-copyable routed target-revocation evidence that preserves the
  caller-selected RSA SHA-256, SHA-384, or SHA-512 OCSP response-signature
  route while keeping both certificate issuer links explicitly fixed to RSA
  SHA-256.
- Share the exact response, target, issuer, extension, cardinality, and
  observation-time validation path between fixed and routed constructors.
- Add routed success and byte-equal distinct-response rejection coverage.

## 0.1.0 - 2026-10-25

- Add non-copyable evidence that binds an issuer-authorized authenticated OCSP
  responder to the exact signed `SingleResponse`, target, and shared issuer of
  one good-and-fresh revocation-policy result.
- Reject byte-equal re-decodes and unsupported outer response extensions.
- Require one unambiguous status entry and revalidate the responder certificate
  at the revocation result's exact observation time.
- Add a public OpenSSL-generated RSA SHA-256 certificate and response fixture
  with signing-operation provenance and no retained private key material.
