# Changelog

## 0.1.0 — 2026-10-01

- Added exclusive exact-snapshot entry and config-schema staging.
- Added trusted absolute runtime-distribution selection and a minimal child environment.
- Added bounded native-helper readiness attestation and fail-closed cleanup.
- Added validated memory, CPU, and descriptor resource-limit arguments.
- Added wall-clock limits, immutable caller snapshot capture, and supervised
  POSIX process-group termination without retaining bare plugin PIDs.
- Added a private Windows supervisor-control pipe so cancellation preserves Job
  and ephemeral-profile cleanup.
- Bound the exported process factory directly to the production
  `forme-plugin-host` request and result types.
- Made cross-platform staging and readiness fixtures honor native Windows path
  and executable-name semantics.
