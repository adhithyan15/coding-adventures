# Changelog

## 0.2.0 - 2026-10-22

### Added

- Added a fail-closed anchor-policy composition that consumes the complete
  reviewed server link and accepts only selected snapshot entries without
  out-of-band name constraints.
- Added distinct non-copyable success evidence plus a payload-free
  `UnsupportedAnchorNameConstraints` failure category.
- Documented that constrained anchors remain unavailable until a
  repository-owned parser and subtree evaluator can apply their constraints.

## 0.1.0 - 2026-10-21

### Added

- Added a non-copyable composition that binds exact target-server policy to
  one reviewed-root RSA SHA-256 issuer link for the same decoded child object.
- Added payload-free mismatch errors, redacted debug output, and focused tests
  for same-object acceptance and byte-equal distinct-object rejection.
- Documented that anchor constraints, path construction, revocation, TLS,
  HTTP, OAuth transport, and network authority remain outside this package.
