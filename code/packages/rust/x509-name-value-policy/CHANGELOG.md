# Changelog

## 0.1.0 — 2026-11-11

- Added a bounded data-driven mapping from validated attribute-type OIDs to
  exact-DER or DirectoryString-scalar comparison.
- Added fail-closed preflight for unconfigured, malformed, and unsupported
  attribute values before structural Name comparison.
- Added payload-free definition and comparison errors with no credential or
  external-effect authority.
