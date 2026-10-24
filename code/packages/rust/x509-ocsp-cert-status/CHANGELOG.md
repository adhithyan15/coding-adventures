# Changelog

## 0.1.0 — 2026-10-24

- Added allocation-free structural RFC 6960 `CertStatus` decoding.
- Enforced exact implicit empty `good` and `unknown` values plus the required
  revocation time and optional explicit reason in `revoked` values.
- Added a typed fixed RFC 5280 CRL reason registry that rejects negative,
  unassigned, non-canonical, and out-of-range values.
- Added redacted local-offset errors and adversarial tag, framing, ordering,
  shared-budget, and diagnostic tests.
