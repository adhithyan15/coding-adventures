# Changelog

## [Unreleased]

### Changed

- `BUILD_windows` now runs the Mix tests on Windows (same as the Unix `BUILD`), now that `elixir/conduit` builds its NIF there, instead of skipping with `ELIXIR_WINDOWS_NIF_DEPENDENCY_UNAVAILABLE`.

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
