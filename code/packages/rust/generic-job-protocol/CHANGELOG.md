# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-20

### Added

- Added `JobRequest<T>`, `JobResponse<U>`, `JobMetadata`, `JobResult`, and
  portable job error types.
- Added a versioned JSON-line wire frame for cross-language process workers.
- Added request/response encode and decode helpers with version, kind, and size
  validation.
- Added tests for success responses, error responses, metadata round trips, and
  malformed frame rejection.
