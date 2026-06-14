# Changelog — conduit-hello (Swift)

## [0.1.0] - 2026-06-13

### Added

- Initial Swift `conduit-hello` demo exercising the full Conduit DSL: `html`,
  `json`, `text`, `respond`, `redirect`, route params, query params, a body echo,
  a `before` filter that short-circuits with `halt`, an `after` header-stamping
  hook, a throwing handler routed to `onError`, and a custom `notFound`.
- `SmokeTests`: launches the demo on an OS-assigned port and asserts on real HTTP
  responses (POSIX-socket client, watchdog).
