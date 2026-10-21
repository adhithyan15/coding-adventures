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
