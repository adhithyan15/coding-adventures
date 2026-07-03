# Changelog — conduit-hello

## [0.1.0] — 2026-06-14

Initial release: C# demo program for CodingAdventures.Conduit (WEB15).

### Added

- `Program.cs`: demo console app with routes (home, health, greet, search, echo,
  redirect, tpot), before-filter for API key check, after-hook for `x-served-by` and
  `x-env` headers, custom not-found and error handlers
- Demonstrates the capture-before-bind pattern: `GetSetting` calls before `Bind()`,
  captured values used inside route closures
- JSON responses via `System.Text.Json.JsonSerializer` (no string interpolation)
- `tests/ConduitHello.Smoke/SmokeTest.cs`: 10 smoke tests for all routes and hooks
- `tools/run-tests.sh`: self-sufficient build script (builds conduit-capi, then tests)
- `BUILD` with `deps=rust/conduit-capi csharp/conduit`
- `BUILD_windows` skip
- `required_capabilities.json`: `ffi:call`, `network:listen`
