# Changelog — conduit-hello (Swift)

## [Unreleased]

### Fixed

- `BUILD_windows`: `where swift` now discards its output with cmd's `>nul
  2>nul` instead of `>/dev/null`. Under `cmd /C` the `/dev/null` redirect
  always failed, so the line always fell through to its "Swift not available"
  echo and the Swift step never ran on Windows, even on runners with Swift
  installed.

## [0.1.0] - 2026-06-13

### Added

- Initial Swift `conduit-hello` demo exercising the full Conduit DSL: `html`,
  `json`, `text`, `respond`, `redirect`, route params, query params, a body echo,
  a `before` filter that short-circuits with `halt`, an `after` header-stamping
  hook, a throwing handler routed to `onError`, and a custom `notFound`.
- `SmokeTests`: launches the demo on an OS-assigned port and asserts on real HTTP
  responses (POSIX-socket client, watchdog).
