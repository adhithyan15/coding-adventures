# Changelog — ircd (Elixir)

## [0.1.0] — 2026-04-12

### Added

- `CodingAdventures.Ircd` entry point module:
  - `main/1` — escript entry point; parses args, starts EventLoop + DriverHandler,
    listens on TCP, blocks until `:stop` message received
  - `stop/0` — send `:stop` to the running ircd (test helper)
  - `parse_args/1` — parse `--host`, `--port`, `--server-name`, `--motd`,
    `--oper-password` flags via `OptionParser`
  - `Config` defstruct with defaults for all fields
- `CodingAdventures.Ircd.DriverHandler` GenServer:
  - Implements `CodingAdventures.IrcNetStdlib.Handler` behaviour
  - State: `%{server_state, framers, loop}` — holds the `IrcServer` state,
    per-connection `Framer` structs, and EventLoop PID
  - `on_connect/2` — registers client in `IrcServer`, creates a `Framer`
  - `on_data/2` — feeds bytes to the Framer, extracts complete lines,
    parses each with `IrcProto.parse/1`, dispatches to `IrcServer.on_message/3`
  - `on_disconnect/1` — calls `IrcServer.on_disconnect/2`, removes Framer
  - `send_responses/2` — serialises `{conn_id, Message}` pairs and sends via
    `EventLoop.send_to/3`
- Mix escript configuration with `main_module: CodingAdventures.Ircd`
- Comprehensive ExUnit test suite — 15 tests, 91.38% coverage:
  - `parse_args/1` unit tests (6 tests)
  - Integration tests for full connect → register → chat → disconnect lifecycle
  - Tests for PRIVMSG, channel JOIN/PRIVMSG, PING/PONG, 421 unknown command
  - `main/1` start-and-stop integration test
