# Changelog — conduit-hello (Perl)

## [0.1.0] - 2026-06-13

### Added

- Initial Perl `conduit-hello` demo exercising the full Conduit DSL: `html`,
  `json`, `text`, `respond`, `halt`, `redirect`, route params, query params,
  request-body echo, a `before` filter that short-circuits, an `after` logging
  filter, a dying handler routed to `on_error`, and a custom `not_found`.
- `t/smoke.t`: launches the demo on an OS-assigned port and asserts on real
  HTTP responses, with an `alarm()` hang guard.
- `$| = 1` autoflush so the "listening on" line is observable before the
  foreground `serve()` call blocks.
