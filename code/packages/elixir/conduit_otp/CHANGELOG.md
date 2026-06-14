# Changelog — conduit_otp

All notable changes to this package will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-27

### Added

- **WEB07: pure-OTP reimplementation of the Conduit web framework.**

#### Package structure

- `mix.exs` — zero external dependencies; requires only Elixir ≥ 1.14 and Erlang/OTP.
- `required_capabilities.json` — `["elixir"]` only (no Rust, no native extensions).
- `BUILD` / `BUILD_windows` — standard `mix deps.get && mix compile && mix test --cover`.

#### Modules (13 total)

- **`OtpApplication`** — implements the OTP `Application` behaviour; starts an empty
  top-level supervisor. Declared in `mix.exs` via `mod:`.
- **`OtpSupervisor`** — `Supervisor` with `:one_for_one` strategy, `max_restarts: 5,
  max_seconds: 10`; starts `RouteTable`, `WorkerSupervisor`, and `Acceptor` in that order.
- **`RouteTable`** — `Agent` holding the compiled `%Application{}` struct; supports
  `snapshot/0` and `hot_reload/1` for zero-restart route updates.
- **`WorkerSupervisor`** — `DynamicSupervisor` that spawns per-connection `Worker`
  processes with `restart: :temporary`.
- **`Acceptor`** — `GenServer` that owns the listen socket; uses a 200 ms timeout on
  `gen_tcp.accept/2` to stay responsive to synchronous calls; normalises
  `{:error, :timeout}` as a non-error poll step.
- **`Worker`** — `GenServer` handling one HTTP/1.1 connection; `send(self(), :process)`
  loop; runs before_filters → route dispatch → after_filters; catches `rescue`
  (exceptions) and `catch` (throws/halts) separately; honours `Connection: close` and
  `keep-alive`.
- **`HttpParser`** — wraps `:erlang.decode_packet/3`; reads request line in `:http_bin`
  mode, headers with `:gen_tcp.recv/3`, body by switching to `{:packet, :raw}`.
- **`Router`** — pure function; `:param` named captures; trailing-slash normalisation;
  first-match wins.
- **`Application`** — identical struct/DSL surface to WEB06 (`get/3`, `post/3`, `put/3`,
  `delete/3`, `patch/3`, `before_filter/2`, `after_filter/2`, `not_found_handler/2`,
  `error_handler/2`, `put_setting/3`, `get_setting/2`).
- **`HaltError`** — `throw`-based non-local control flow (`halt/1,2,3`, `redirect/1,2`);
  CRLF injection guard in `redirect/1`.
- **`HandlerContext`** — response helpers (`html/1,2`, `json/1,2`, `text/1,2`,
  `respond/2,3`); delegates `halt/*` and `redirect/*` to `HaltError`.
- **`Request`** — immutable struct; `from_parsed/4,5` (for internal use by Worker) and
  `from_env/1` (for compatibility with WEB06 env maps); query-string parser; `json_body!/1`.
- **`Server`** — public façade; `start_link/2`, `serve/1`, `stop/1`, `local_port/1`,
  `running?/1`; uses `unique_integer` to give each server instance distinct process names.

#### Tests (148 passing)

- `application_test.exs` — 23 tests: DSL chainability, handler IDs, settings, immutability.
- `halt_error_test.exs` — 9 tests: throw semantics, CRLF injection guards.
- `handler_context_test.exs` — 15 tests: all response helpers, halt/redirect delegation.
- `http_parser_test.exs` — 12 tests: request line parsing, header decoding, `decode_packet/1`.
- `request_test.exs` — 22 tests: `from_parsed`, `from_env`, query params, `json_body!`.
- `router_test.exs` — 14 tests: exact match, named captures, method matching, no-match.
- `route_table_test.exs` — 8 tests: Agent start, snapshot, hot_reload.
- `server_test.exs` — 30 tests: E2E via `:httpc` — GET/POST/PUT/PATCH/DELETE, before/after
  filters, redirect, halt, not_found, error handler, query params, metadata.
- `worker_test.exs` — 11 tests: raw TCP requests for `Connection: close`, filter pass-through,
  throw handling, body reading, disconnect recovery.
- `conduit_otp_test.exs` — 1 test: umbrella module smoke test.

#### Coverage

- Total: 80.00% (threshold: 80%).
- 100% on: `Application`, `HandlerContext`, `HaltError`, `OtpApplication`, `OtpSupervisor`,
  `Router`, `CodingAdventures.ConduitOtp`.
