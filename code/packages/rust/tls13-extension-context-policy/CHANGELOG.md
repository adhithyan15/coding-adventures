# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-29

### Added

- Added a bounded caller-owned extension-context rule policy.
- Recorded all 22 RFC 8446 Table 1 placements in an auditable standard rule
  table without enabling extension semantics.
- Added future-client validation for outbound ClientHello and six
  server-originated message contexts.
- Enforced ClientHello `pre_shared_key` ordering and response-to-offer binding,
  including the HelloRetryRequest `cookie` exception.
- Added stable payload-free failures, allocation-free operation, explicit
  readiness boundaries, and focused integration coverage.
