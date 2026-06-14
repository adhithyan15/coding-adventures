# Changelog — CodingAdventures.Conduit

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
