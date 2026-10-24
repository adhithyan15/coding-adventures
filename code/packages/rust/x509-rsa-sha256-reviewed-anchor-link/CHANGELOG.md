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
- Added a construction-bound fixed two-link composition that retains one exact
  target-to-intermediate path-length transition, the same construction's
  authenticated terminal adjacency, and reviewed-root selection for that exact
  terminal certificate.
- Added real public-chain coverage plus rejection tests for independently
  constructed paths, a nonterminal second projection, and reviewed-root
  evidence from a byte-equal re-decode.

### Security

- The two-link composition retains both adjacent-link proofs and the advanced
  path-length state, but grants no target policy, anchor-constraint, general
  path, revocation, TLS, HTTP, OAuth transport, or network authority.
- The construction-bound composition additionally requires one exact two-link
  path shape and construction-object identity. Its initial path-length state
  remains caller-supplied, and it grants no generalized validation or trust.
