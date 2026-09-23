# Changelog

## 0.1.0 — 2026-09-20

- Added allocation-free RFC 5280 `Name`, RDN, and attribute decoding.
- Enforced non-empty DER-ordered RDN sets plus explicit RDN and total-attribute
  bounds while retaining borrowed attribute OIDs and exact opaque values.
- Added malformed nesting, ordering, shared-limit, bound, and redacted-error
  tests.
