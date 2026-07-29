# Changelog — conduit-hello (Lua)

## Unreleased

### Fixed
- Run the E2E server in a dedicated Lua child process using the production
  foreground `serve()` path. The previous in-process `serve_background()` test
  allowed the Busted runner and native request threads to access one
  `lua_State` concurrently, causing a segmentation fault on Linux CI.

## 1.0.0 — 2026-06-15

Standardise the Lua demo to match every other Conduit `conduit-hello` program
(which all ship a BUILD, README, CHANGELOG, and a `tests/` suite). Previously the
directory held only `hello.lua`, so the demo was absent from the build graph and
untested.

### Added
- `BUILD` / `BUILD_windows` — declare `deps=lua/conduit`, compile the conduit
  package's `conduit_native` cdylib, install `luasocket`, and run the demo's
  E2E tests via busted.
- `tests/test_hello.lua` — end-to-end tests that load the demo's `build_app()`,
  serve it on an ephemeral port in the background, and drive all eight routes
  (`/`, `/hello/:name`, `/echo`, `/redirect`, `/halt`, `/down`, `/error`,
  unmatched → 404) over real HTTP. Self-skips when `luasocket` is unavailable,
  mirroring the conduit library's server suite.
- `README.md` — routes table, run instructions, structure, and test notes.

### Changed
- `hello.lua` — refactored so the application is built in a `build_app()`
  factory (returns a configured `conduit.Application`, starts no server) with a
  `serve()` helper. A bottom-of-file guard runs the foreground server only when
  the file is invoked directly (`lua hello.lua`); when loaded as a module it just
  returns the factory, so the tests can construct and drive the same app. Routes
  and behaviour are unchanged.
