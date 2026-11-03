# Changelog

All notable changes to the `rpc` Swift package are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [Unreleased]

### Fixed

- `BUILD_windows`: `where swift` now discards its output with cmd's `>nul
  2>nul` instead of `>/dev/null`. Under `cmd /C` the `/dev/null` redirect
  always failed, so the line always fell through to its "Swift not available"
  echo and the Swift step never ran on Windows, even on runners with Swift
  installed.

## [0.1.0] - 2026-04-11

### Added

- `RpcId` - string-or-integer request ids with literal conformances
- `RpcRequest`, `RpcResponse`, `RpcErrorResponse`, and `RpcNotification`
- `RpcMessage` sum type for codec boundaries
- `RpcCodec` and `RpcFramer` protocols
- `RpcServer` request/notification dispatch loop
- `RpcClient` blocking request API with notification handling
- `RpcErrorCodes` constants and `RpcErrorResponse` convenience builders
- Swift test coverage for request dispatch, notifications, errors, and client
  correlation behavior
