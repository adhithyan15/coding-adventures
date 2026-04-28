# Changelog

All notable changes to `coding_adventures_conduit` are documented here.

## 0.1.0 — 2026-04-26

Initial release. Elixir port (WEB06) of the Conduit web framework.

### Added

- **Application DSL** (`Conduit.Application`): immutable struct accumulating
  routes (`get`/`post`/`put`/`delete`/`patch`), `before_filter`, `after_filter`,
  `not_found_handler`, `error_handler`, and string-keyed settings.
- **HandlerContext helpers** (`Conduit.HandlerContext`): `html/2`, `json/2`,
  `text/2`, `respond/3`, plus `halt/1..3` and `redirect/1..2` re-exported
  from `Conduit.HaltError`. Uses Elixir 1.18+'s built-in `JSON` module
  with a hand-rolled fallback for older versions.
- **Request struct** (`Conduit.Request`): projects the Rust env map onto
  named fields (`method`, `path`, `params`, `headers`, etc.) plus
  `json_body!/1` with a 10 MiB payload-size guard.
- **HaltError** (`Conduit.HaltError`): `throw {:conduit_halt, status, body, headers}`
  is the cheap, idiomatic mechanism for Sinatra-style halts on BEAM.
- **Server orchestration** (`Conduit.Server`): `start_link/2`, `serve/1`,
  `serve_background/1`, `stop/1`, `local_port/1`, `running?/1`.
- **Dispatcher** (`Conduit.Dispatcher`): single GenServer that owns the
  compiled `handlers` map and processes `{:conduit_request, ...}` messages.
  Catches `:throw` (HaltError) and `:rescue`-able exceptions, formatting
  them into either a real response or the bare-500 sentinel for
  re-dispatch through `error_handler`.
- **Native bridge** (`Conduit.Native`): 15 NIF stubs loaded via `@on_load`.
  `server_serve/1` is flagged `ERL_NIF_DIRTY_JOB_IO_BOUND` so it doesn't
  starve the BEAM scheduler.
- **Rust cdylib** (`native/conduit_native/`): cross-thread dispatch via
  `enif_send` + a per-request slot table with `Mutex`/`Condvar`.
  HTTP status codes clamped to 100–599 in the response parser.
- **`erl-nif-bridge` extensions**: maps (`enif_make_new_map`,
  `enif_make_map_put`, `enif_get_map_value`, map iterators), pids
  (`ErlNifPid`, `enif_self`, `enif_get_local_pid`, `enif_make_pid`),
  and `enif_send` (off-scheduler-thread variant). Plus binary helpers
  (`str_to_binary`, `binary_to_bytes`, `binary_to_string`).
- **40+ ExUnit tests** across halt_error, handler_context, request,
  application, and server (E2E via real TCP + httpc).
- **`conduit-hello` demo** (`programs/elixir/conduit-hello`): 8-route
  Sinatra-style application with 15+ integration tests.

### Out of scope

- Per-request worker processes (deferred to WEB07 — Conduit OTP, pure-OTP
  reimplementation that uses one process per connection).
- Windows NIF support (BEAM exports erts.dll in a way that requires more
  explicit linkage).
