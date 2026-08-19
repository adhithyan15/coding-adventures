# Changelog — conduit-hello (Swift)

## Unreleased

### Fixed

`BUILD_windows` now carries the `# build-tool: deps=swift/conduit rust/conduit-capi`
declaration. The build tool reads that directive out of whichever BUILD file it
selects for the current platform, so a directive present only in `BUILD` left both
edges missing from the dependency graph on Windows.

## [0.1.0] - 2026-06-13

### Added

- Initial Swift `conduit-hello` demo exercising the full Conduit DSL: `html`,
  `json`, `text`, `respond`, `redirect`, route params, query params, a body echo,
  a `before` filter that short-circuits with `halt`, an `after` header-stamping
  hook, a throwing handler routed to `onError`, and a custom `notFound`.
- `SmokeTests`: launches the demo on an OS-assigned port and asserts on real HTTP
  responses (POSIX-socket client, watchdog).
