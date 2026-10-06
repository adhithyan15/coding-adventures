# Changelog

## Unreleased

## 1.0.0 — 2026-10-05

- Join the aligned stable Forme `1.0.0` package line targeting kernel API v2; see the [`forme-types` migration guide](../forme-types/MIGRATION-v2.md) for the breaking `RenderedPage` provenance and stage/plugin version changes.

- Align the host-owned default descriptor budget with FM02's specified 256 so
  standard Node and Python runtimes are not terminated during normal Windows
  startup when a plugin manifest omits resource overrides.

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
- Kept per-platform coverage gates deterministic while testing Windows system
  root validation and delegating POSIX cleanup paths to native platform gates.
- Added the trusted native launcher's bounded exit result to attestation-failure
  diagnostics without exposing plugin-controlled output.
- Added bounded trusted-launcher stderr to failed pre-attestation diagnostics
  so unavailable OS primitives retain their actionable native error.
- Supplied the validated `LOCALAPPDATA` bootstrap required for Windows to
  construct and rewrite an AppContainer environment without copying ambient
  application variables.
- Passed the validated manifest runtime kind to native launchers for
  runtime-specific, fail-closed bootstrap behavior.
