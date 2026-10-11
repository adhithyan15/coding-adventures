# Changelog

## Unreleased

### Security

- The HTTP preflight now refuses URL paths and queries that contain anything
  outside RFC 3986 `pchar`, `/` and `?`, or a malformed `%` escape. Before
  this, the URL parser passed control characters, spaces and non-ASCII bytes
  through untouched, and they went verbatim onto the request line. So a CR or
  LF in a URL could end the request line early and add headers of its own,
  defeating any header policy the caller enforced. Found by the security
  review of `chief-of-staff-net-fetch` (D18V), whose URLs come from a model.

## [0.1.0] - 2026-05-14

### Added

- Added Rust operation envelope primitives: `OperationResult`, `ResultFactory`,
  `OperationScope`, `OperationOutcome`, `OperationError`, and `start_new`.
- Added panic capture with optional rethrow through `panic_on_unexpected`.
- Added `OperationHttpClient`, an operation-side HTTP preflight wrapper fed by
  generated code from `required_capabilities.json` that refuses undeclared HTTPS
  domains before transport callbacks can run.
