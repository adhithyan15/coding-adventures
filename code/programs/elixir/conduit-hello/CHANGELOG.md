# Changelog

## 0.1.0 — 2026-04-26

Initial release. Mirrors the Ruby/Python/Lua/TypeScript conduit-hello
demos route-for-route.

### Added

- Eight-route Sinatra-style demo: `/`, `/hello/:name`, `/echo`,
  `/redirect`, `/halt`, `/down`, `/error`, plus custom `not_found` and
  `error_handler` fallbacks.
- 15 ExUnit integration tests covering every route via real TCP and
  Erlang's `:httpc` HTTP client.
- `escript: [main_module: ConduitHello]` so you can run it via
  `mix escript.build && ./conduit_hello`.
