# Changelog

## 0.1.0 - 2026-10-21

### Added

- Added a non-copyable composition that binds the strongest repository-owned
  RSA SHA-256 issuer-link evidence to reviewed-root selection evidence for the
  same exact decoded issuer certificate.
- Added payload-free mismatch errors and redacted debug output.
- Added an independent public ISRG Root X1 interoperability vector covering
  successful self-signature verification and exact reviewed-snapshot binding.
- Added a distinct-object rejection test proving that byte-equal certificates
  cannot mix independently produced evidence.
- Added a non-copyable two-link composition that joins one authenticated
  child-to-intermediate path-length transition to one authenticated
  intermediate-to-reviewed-root link only when both retain the identical
  decoded intermediate object.
- Added success and byte-equal distinct-object rejection coverage for the
  reviewed intermediate path composition.

### Security

- The two-link composition retains both adjacent-link proofs and the advanced
  path-length state, but grants no target policy, anchor-constraint, general
  path, revocation, TLS, HTTP, OAuth transport, or network authority.
