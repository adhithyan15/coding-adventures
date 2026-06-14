# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Added

- Deterministic retry policies with bounded backoff, transient status matching,
  request-plan propagation, and query filters for supervised local HTTP calls.
- `LocalHttpEndpointSummary` and `summarize_local_http_endpoints()` for compact
  endpoint inventory shape across scheme, TLS, base path, metadata, and unique
  host/bridge/integration coverage.
- `LocalHttpRequestPlanSummary` and `summarize_local_http_request_plans()` for
  aggregate method, auth, retry, body, vault, and timeout shape before a runtime
  executes local HTTP requests.

## [0.1.0] - 2026-05-08

### Added

- Local HTTP endpoint descriptors for bridge-bound integrations.
- Request templates and deterministic request plans with timeout and idempotency
  metadata.
- Vault-backed bearer, basic, custom header-token, and client-certificate auth
  descriptors.
- Header conflict detection and content-type validation for body-bearing
  requests.
- Endpoint and request-plan query options for filtering by integration, bridge,
  scheme, TLS policy, host, method, auth kind, idempotency, body presence,
  vault requirements, timeout, sort order, and bounded result count.
