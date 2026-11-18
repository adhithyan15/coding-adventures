# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-18

### Added

- Server-role-only transcript evidence acceptance before RSA work.
- Reviewed-path leaf SubjectPublicKeyInfo derivation and data-driven RSA key
  policy.
- Non-copyable success evidence retaining the complete reviewed server path and
  exact selected-transcript RSA-PSS signature verdict.
- Stable redacted failure categories and explicit non-readiness boundaries.
- End-to-end public-path fixtures covering role-first rejection, caller-buffer
  non-mutation, exact reviewed-leaf key derivation, signature verification,
  and retained proof identity.
