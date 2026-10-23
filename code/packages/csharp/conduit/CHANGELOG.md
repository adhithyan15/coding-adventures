# Changelog — CodingAdventures.Conduit

## [Unreleased]

### Fixed

- **Double free of every handler `GCHandle`.** `Application.Dispose()` freed
  the registered handles itself after `conduit_app_free` (or a consumed
  `Bind`) had already freed each one through Rust's `CtxFree`. `GCHandle` is
  a struct, so our copies still reported `IsAllocated`, and .NET's `Free()`
  does not detect a second free. Each slot went back to the handle table
  twice, so later registrations could share a slot and a trampoline would
  dispatch to another delegate. That caused the intermittent wrong statuses
  the test suite blamed on a "cold-start race", and the "Test host process
  crashed" abort seen on CI (PR #18005, shard 15). `Dispose` now leaves the
  handles to Rust. New test `Dispose_ReleasesEachHandlerHandleExactlyOnce`.
- **Double free of the after-hook's `current` response.** `AfterFn` freed
  `current` and then, if the hook threw (or the delegate cast failed), freed
  it again in its `catch`, which is a native double free that aborts the
  process. `current` is now cleared once freed.
- **A halt response that cannot be marshalled no longer kills the process.**
  `h.Response.ToNative()` in the trampolines' `HaltException` handlers could
  throw (for example on an out-of-range status) out of an
  `[UnmanagedCallersOnly]` method, which terminates the process. It now
  falls back to a safe 500 (`HaltToNative`).
- Removed the now-unused `_handles` list.

## [0.1.0] — 2026-06-14

Initial release: C# P/Invoke binding for the conduit-capi C ABI (WEB15).

### Added

- `Native` internal class with `[DllImport]` declarations for all conduit-capi functions
- `NativeLibrary.SetDllImportResolver` for locating `libconduit_capi.so/.dylib` via
  `CONDUIT_CAPI_PATH` environment variable (set by build script) or OS default search
- `Response` sealed class with `Html`, `Json`, `Text`, `Respond`, `Redirect` factory
  methods; `WithHeader` for fluent header addition
- `HaltException` for non-local exit from handlers (equivalent to Sinatra's `halt`)
- `Request` sealed class with `Method`, `Path`, `QueryString`, `ContentType`,
  `RemoteAddr`, `Error`, `Param`, `Query`, `Header`, `Body`, `BodyString` accessors
- `Trampolines` static unsafe class with `[UnmanagedCallersOnly(CallConvCdecl)]` methods
  for `HandlerFn`, `BeforeFn`, `AfterFn`, `CtxFreeFn` — bridging C function pointers
  to managed C# delegates via `GCHandle`
- `Application` sealed class: fluent builder with `Get`/`Post`/`Put`/`Delete`/`Patch`/
  `Route`, `Before`, `After`, `NotFound`, `OnError`, `Set`, `GetSetting`, `Bind`
- `Server` sealed class: `Serve` (blocking), `ServeBackground`, `Stop`, `LocalPort`,
  `IsRunning`, `Dispose` (IDisposable with Stop + Free)
- `BeforeWrapper` box type for before-filter callbacks using the dedicated `BeforeFn`
  trampoline (returns `IntPtr.Zero` = continue; non-zero = short-circuit)
- 35 unit and E2E tests: 9 Response factory, 9 Application config, 3 Server lifecycle,
  14 end-to-end HTTP via `HttpClient` + `ServeBackground` + 30-second watchdog timer
- `tools/run-tests.sh`: self-sufficient build script that builds conduit-capi via
  `cargo build --release -q`, sets `CONDUIT_CAPI_PATH`, then runs `dotnet test`
- `BUILD` with `deps=rust/conduit-capi` directive
- `BUILD_windows` skip (cdylib requires cross-compile setup)
- `required_capabilities.json`: `ffi:call`, `network:listen`
