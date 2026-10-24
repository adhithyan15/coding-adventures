# Changelog — CodingAdventures.Conduit.FSharp

## Unreleased

- Fix a double free of every handler `GCHandle`: `Application`'s `Dispose`
  freed the handles again after Rust's `CtxFree` had freed them (via
  `conduit_app_free` or a consumed `bind`). The slot returned to the handle
  table twice, so later registrations could share it and dispatch to the
  wrong function. `Dispose` now leaves them to Rust, the same fix as the C#
  binding.
- Fix a double free of `current` in the after-hook trampoline when the hook
  throws: it is now freed only while still owned.
- A halt whose response cannot be marshalled now becomes a safe 500
  (`haltToNative`) instead of throwing out of a native callback.
- Registration reads `app.Ptr` (which throws once the app is bound) before
  allocating the handler's `GCHandle`, so a call after `bind` leaks nothing.
  The unused `_handles` list is gone.

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
