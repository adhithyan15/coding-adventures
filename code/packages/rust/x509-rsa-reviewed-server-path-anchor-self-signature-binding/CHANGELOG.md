# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-13

### Added

- Exact-object binding between the strongest accumulated reviewed RSA
  server-path policy and complete reviewed-anchor self-signature evidence.
- Non-copyable evidence retaining both complete inputs after the identity
  check succeeds.
- Stable payload-free mismatch errors and redacted `Debug` output.
- Tests for successful real USERTrust RSA root self-signature composition and
  rejection of a byte-equal independently decoded root.
- POSIX and Windows build fronts plus explicit zero-capability metadata.
