# Changelog — CodingAdventures.Conduit.FSharp

## 0.1.0 — 2026-06-14

Initial release (WEB16).

- F# P/Invoke binding for the `conduit-capi` Rust cdylib (WEB12).
- Functional, pipe-friendly API: `Application.create() |> Application.get "/" h |> Application.bind "127.0.0.1" 3000us`.
- `Response` module: `html`, `json`, `text`, `respond`, `redirect`, `withHeader`, `withStatus`.
  - Note: F# module functions cannot have optional parameters (FS0718), so `html/json/text/redirect`
    default to 200/302; use `|> Response.withStatus N` to override (e.g. `Response.html body |> Response.withStatus 201`).
    Spec updated to reflect this divergence from the original optional-parameter design.
- `Request` type: `Method`, `Path`, `QueryString`, `ContentType`, `RemoteAddr`, `Error`, `Param`, `Query`, `Header`, `Body`, `BodyString`.
- `Application` module: `create`, `set`, `getSetting`, `get`, `post`, `put`, `delete`, `patch`, `route`, `before`, `after`, `notFound`, `onError`, `bind`.
- `Server` type: `LocalPort`, `IsRunning`, `Serve`, `ServeBackground`, `Stop`, `IDisposable`.
- `HaltException` for non-local exits from handlers.
- 40 tests across 4 groups; ≥ 80% line coverage.
- 30-second E2E watchdog guard.

Security properties:
- No `File.Exists` TOCTOU in native library resolution.
- Null ctx guards in all three trampolines.
- `nuint→int` bounds checks before Array allocation.
- Sanitised bind-failure message (raw error → stderr, generic message → exception).
- Status code range validation `[100, 999]` before `uint16` cast.
