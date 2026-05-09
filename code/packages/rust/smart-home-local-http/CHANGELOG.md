# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Added

- Deterministic retry policies with bounded backoff, transient status matching,
  request-plan propagation, and query filters for supervised local HTTP calls.

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
