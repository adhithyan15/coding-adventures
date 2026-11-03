# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Fixed

- `BUILD_windows`: `where swift` now discards its output with cmd's `>nul
  2>nul` instead of `>/dev/null`. Under `cmd /C` the `/dev/null` redirect
  always failed, so the line always fell through to its "Swift not available"
  echo and the Swift step never ran on Windows, even on runners with Swift
  installed.

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
