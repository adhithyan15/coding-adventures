# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- Initial release of the JSON-RPC 2.0 package for Swift.
- `Request`, `Response`, `Notification`, `ResponseError` message types.
- `MessageReader` for reading Content-Length-framed JSON-RPC messages.
- `MessageWriter` for writing Content-Length-framed JSON-RPC messages.
- `Server` combining reader + writer with method dispatch.
- Standard JSON-RPC 2.0 error codes (ParseError, InvalidRequest, MethodNotFound, InvalidParams, InternalError).
- `parseMessage()` and `messageToMap()` conversion functions.
- Comprehensive test suite covering all message types, framing, and server dispatch.
