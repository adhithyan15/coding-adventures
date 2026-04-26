# Changelog — coding-adventures-conduit (Lua)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [1.0.0] — 2026-04-24

### Added

- **`conduit/conduit_native` (Rust cdylib)** — Lua 5.4 C extension backed by
  `web-core`. Exports `new_app`, `app_add_route`, `app_add_before`,
  `app_add_after`, `app_set_not_found`, `app_set_error_handler`,
  `app_set_setting`, `app_get_setting`, `new_server`, `server_serve`,
  `server_serve_background`, `server_stop`, `server_local_port`,
  `server_running`, and `server_dispose`.

- **`conduit/json.lua`** — Minimal JSON encoder/decoder. Supports strings,
  numbers, booleans, nil/null, arrays, and objects. No third-party LuaRocks
  dependencies.

- **`conduit/halt.lua`** — `HaltError` table helpers: `new(status, body,
  headers)`, `raise(status, body, headers)`, and `is_halt_error(err)`.

- **`conduit/request.lua`** — `Request` class wrapping the Rust env table with
  typed accessors: `method()`, `path()`, `params()`, `query()`, `headers()`,
  `body()`, `content_type()`, and `json_body()` (raises HaltError 400 on
  invalid JSON).

- **`conduit/handler_context.lua`** — `HandlerContext` class inheriting from
  `Request` and adding response helpers: `html()`, `json()`, `text()`,
  `redirect()`, `halt()`. Also exports module-level functions used as
  `conduit.html`, `conduit.json`, etc.

- **`conduit/application.lua`** — `Application` class with route registration
  (`get`, `post`, `put`, `delete`, `patch`), lifecycle filters (`before`,
  `after`), special handlers (`not_found`, `error_handler`), settings
  (`set` / `get`), and route introspection (`routes()`).

- **`conduit/server.lua`** — `Server` class wrapping the native server userdata:
  `serve()`, `serve_background()`, `stop()`, `local_port()`, `running()`,
  `dispose()`.

- **`conduit/init.lua`** — Main entry point loaded by `require("conduit")`.
  Re-exports all public API: module-level helpers, `Application`, `Server`,
  `halt`, `is_halt_error`.

- **`coding-adventures-conduit-1.0.0-1.rockspec`** — LuaRocks specification for
  the pure-Lua DSL layer.

- **Tests (`tests/`)** — 60+ unit and integration tests using the `busted`
  framework:
  - `test_halt.lua` — HaltError construction, raise, is_halt_error
  - `test_request.lua` — Request accessor methods, json_body parsing
  - `test_handler_context.lua` — Response helpers, request delegation
  - `test_application.lua` — Route registration, filters, settings, introspection
  - `test_server.lua` — Full E2E tests via real TCP (luasocket)

- **`code/programs/lua/conduit-hello/hello.lua`** — 8-route demo program
  exercising every framework feature.

### Fixed

- **Lua GC premature collection of `LuaConduitApp` on Linux** — On Linux
  (glibc), Lua's GC is more aggressive than on macOS. After `setup()` returns,
  the local `app` variable goes out of scope, leaving `LuaConduitApp` userdata
  unreachable. The GC would then fire `app_gc`, calling `luaL_unref` on every
  handler registry slot while the server's Rust closures still held those slot
  integers. Subsequent HTTP requests would get nil from `lua_rawgeti` and return
  500. Fix: `lua_new_server` now pins the app userdata into the Lua registry
  (`luaL_ref`) and stores the reference in `LuaConduitServer.app_ref`. The pin
  is released in `server_gc` after the server has fully stopped and all Rust
  closures are guaranteed to never fire again.

### Implementation notes

- **Threading model**: `lua_State` is not thread-safe; all Lua callbacks are
  serialised through an `Arc<Mutex<()>>` ("Lua lock"). Web-core's I/O threads
  each acquire the lock before calling `lua_pcall`.

- **`serve_background()`**: spawns a Rust `std::thread` to run the server loop
  without blocking the calling Lua thread. Used by `test_server.lua`.

- **`ThreadSafePtr<T>`**: a Rust newtype wrapper with `unsafe impl Send + Sync`
  used to allow closures capturing `*mut lua_State` (which is `!Send + !Sync`)
  to satisfy `web-core`'s `Send + Sync` closure bounds. Safety is guaranteed by
  the Lua lock.

- **`build.rs`**: sets `-undefined dynamic_lookup` (macOS),
  `--allow-shlib-undefined` (Linux), or links `lua54.lib` (Windows) so the
  cdylib can be built without statically linking liblua.
