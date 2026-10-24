# Changelog

## 0.1.0 — 2026-10-24

- Added fail-closed classification for caller-supplied good, revoked, and
  unknown certificate-status observations.
- Required an ordered inclusive `thisUpdate..nextUpdate` interval around one
  explicit validated caller-owned time.
- Added non-copyable, exact-observation policy acceptance and stable
  payload-free failure categories without claiming authenticated revocation
  evidence or transport readiness.
