# Changelog

## 0.1.0 — 2026-11-17

### Added

- Selected-group-only routing to the repository-owned X25519 primitive.
- Exact peer-key and derived-secret length enforcement through the selected
  named-group policy.
- Stable payload-free failures for unsupported groups, length policy, and peer
  public-key rejection.
- Non-cloneable caller-owned shared-secret views with exact wipe-on-drop.
- RFC 7748 success, unsupported-route, invalid-length, low-order, redaction,
  unchanged-output, and wipe-on-drop coverage.
