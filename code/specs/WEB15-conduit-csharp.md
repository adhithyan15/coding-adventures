# WEB15 — Conduit C# P/Invoke Binding

## Position in the Stack

```
┌─────────────────────────────────────────────────────────────┐
│  Application code (Program.cs, conduit-hello)               │
├─────────────────────────────────────────────────────────────┤
│  CodingAdventures.Conduit  (this package, C# / P/Invoke)    │
├─────────────────────────────────────────────────────────────┤
│  conduit-capi  (WEB12, Rust extern "C" static lib)          │
├─────────────────────────────────────────────────────────────┤
│  web-core  (WEB08, Rust engine)                             │
└─────────────────────────────────────────────────────────────┘
```

`CodingAdventures.Conduit` is a .NET 9 class library that wraps the `conduit-capi`
C ABI via **P/Invoke** — .NET's Platform Invocation Services. It provides an
idiomatic, fluent C# API with RAII resource management, GCHandle-pinned delegate
lifetime control, and type-safe response helpers.

## Design Goals

1. **Idiomatic C#** — fluent builder pattern, `using` IDisposable, nullable
   annotations, `System.Text.Json` for structured payloads.
2. **Correct delegate lifetimes** — .NET's GC can collect delegates if nothing
   keeps a strong reference. Every callback is stored in a `GCHandle` (allocated,
   not pinned — no need to pin a reference type) and freed only when the native
   side calls `ctx_free`.
3. **No unsafe in user code** — all `unsafe` code lives inside `Conduit.cs`.
   Users interact with entirely managed types.
4. **Self-sufficient build** — `tools/run-tests.sh` builds `conduit-capi` via
   `cargo build --release -q` before running `dotnet test`, so the Rust `.so`/`.dylib`
   is always present regardless of whether `deps=` ordering ran first.

## P/Invoke Fundamentals

### DllImport vs LibraryImport

We use `[DllImport]` (classic, works everywhere) rather than `[LibraryImport]`
(source-generated, .NET 7+). Both are available on .NET 9; `[DllImport]` requires
less boilerplate for complex signatures with `nuint` parameters.

### Library Loading

`conduit-capi` builds as both a staticlib (`.a`) and a cdylib (`.so`/`.dylib`/`.dll`).
P/Invoke requires a **shared library** — it cannot link against `.a` files. We load
the cdylib.

.NET resolves `[DllImport("conduit_capi")]` as:
- Linux: `libconduit_capi.so`
- macOS: `libconduit_capi.dylib`
- Windows: `conduit_capi.dll`

The `Native` class uses `NativeLibrary.SetDllImportResolver` to locate the library:
1. If `CONDUIT_CAPI_PATH` env var is set, load that exact path (used by `run-tests.sh`).
2. Otherwise fall through to OS default search paths (`LD_LIBRARY_PATH` / `DYLD_LIBRARY_PATH`).

### Callback Lifetime Problem

Native C code holds function pointers (and `ctx` opaque pointers) that must remain
valid as long as the `ConduitApp`/`ConduitServer` is alive. If .NET's GC collects a
delegate, the function pointer becomes dangling.

**Solution — `GCHandle.Alloc`:**

```
                  C# managed heap                    C native heap
                 ┌──────────────┐                   ┌──────────────┐
  lambda fn  ──► │  HandlerBox  │◄── GCHandle ───►  │  ctx (IntPtr)│
                 └──────────────┘   (strong root)   └──────────────┘
                        ▲                                   │
                        └───────────────────────────────────┘
                              trampoline recovers via
                         GCHandle.FromIntPtr(ctx).Target
```

1. `GCHandle.Alloc(closure)` creates a strong root that prevents GC collection.
2. `GCHandle.ToIntPtr(handle)` gives an `IntPtr` we pass as the `void* ctx`.
3. The static `[UnmanagedCallersOnly]` trampoline recovers the closure:
   `(HandlerFunc)GCHandle.FromIntPtr(ctx).Target!`
4. `ctx_free` trampoline calls `GCHandle.FromIntPtr(ctx).Free()`, releasing the root
   and allowing the closure to be GC'd.

This is exactly analogous to Go's `cgo.Handle` pattern from WEB14.

### UnmanagedCallersOnly Trampolines

`[UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]` declares a
static method as directly callable from C with the C calling convention:

```csharp
[UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
private static IntPtr HandlerTrampoline(IntPtr ctx, IntPtr req) {
    try {
        var fn = (HandlerFunc)GCHandle.FromIntPtr(ctx).Target!;
        return fn(new Request(req)).ToNative();
    } catch (HaltException h) { return h.Response.ToNative(); }
      catch (Exception ex)    { ReportError(ex); return IntPtr.Zero; }
}
```

Rules:
- Must be `static`
- Parameters must be unmanaged (blittable) types — `IntPtr`, `ushort`, `nuint` etc.
- Must not let exceptions escape — must catch all and handle

Function pointers are obtained as static fields:
```csharp
internal static readonly IntPtr Handler = (IntPtr)(void*)&HandlerTrampoline;
```

## Public API Surface

```csharp
// Response — immutable value built by factory methods
Response.Html(string body, int status = 200)
Response.Json(string body, int status = 200)
Response.Text(string body, int status = 200)
Response.Respond(int status, string body, params (string Name, string Value)[] headers)
Response.Redirect(string location, int status = 302)  // throws on CR/LF in location

// Request — borrowed view, valid only inside handler
req.Method           // "GET", "POST", …
req.Path             // "/api/users/42"
req.QueryString      // "foo=bar&baz=qux"
req.ContentType      // "application/json" or ""
req.RemoteAddr       // "127.0.0.1:54321"
req.Error            // non-empty only in OnError handler
req.Param("name")    // route parameter or null
req.Query("q")       // query string value or null
req.Header("accept") // header value (case-insensitive) or null
req.Body()           // byte[]
req.BodyString()     // UTF-8 decoded body

// Application — builder (fluent)
var app = new Application();
app.Set("key", "value")             // set a string setting
app.Get("value")                    // retrieve setting (before Bind only)
app.Get("/path/:param", handler)    // route registration
app.Post("/path", handler)
app.Put("/path", handler)
app.Delete("/path", handler)
app.Route("PATCH", "/path", handler)
app.Before(beforeFilter)            // null return = continue; Response = short-circuit
app.After(afterTransformer)         // receives current Response, returns (possibly mutated) Response
app.NotFound(handler)
app.OnError(handler)
using var server = app.Bind("127.0.0.1", 3000);  // consumes app

// Server — IDisposable RAII
server.Serve()            // blocks; 0 = ok
server.ServeBackground()  // non-blocking; 0 = ok
server.Stop()
server.LocalPort          // ushort — actual bound port (useful with port 0)
server.IsRunning          // bool

// Halt — throw inside a handler to short-circuit with a specific response
throw new HaltException(Response.Text("Forbidden", 403));
```

## Memory Ownership (C# perspective)

| Object          | Owned by                              | Freed when                                 |
|-----------------|---------------------------------------|--------------------------------------------|
| `Application`   | C# (IDisposable)                      | `Bind()` consumes it; or `Dispose()`       |
| `Server`        | C# (IDisposable / `using`)            | `server.Dispose()` → `conduit_server_free` |
| `Request`       | Rust (borrowed)                       | Handler returns — do NOT store the ref     |
| `Response`      | C# until returned from handler        | Returned responses owned by Rust           |
| `GCHandle`s     | `Application._handles` list           | `ctx_free` trampoline calls `.Free()`      |

## Security Properties

All trust-boundary enforcement happens in Rust (`conduit-capi`):
- Header injection (CR/LF in names/values) → dropped
- Status code clamped to 100–599
- Body bytes passed through as-is (no re-encoding)
- Panic isolation — panics in Rust handlers are caught and logged

C# layer adds:
- Redirect location CR/LF check (throw `ArgumentException` — belt-and-suspenders)
- Panic-message sanitisation: strip ASCII control chars, cap at 512 bytes, log server-side only
- All structured JSON responses built with `System.Text.Json` (no string interpolation)

## Test Coverage (30+ tests)

| Category                           | Count |
|------------------------------------|-------|
| Response factory unit tests        | 9     |
| Application settings/configuration | 5     |
| Application route chaining         | 4     |
| Bind / server lifecycle            | 3     |
| End-to-end HTTP (ServeBackground)  | 14    |
| **Total**                          | **35**|

E2E tests use `HttpClient` against a live `ServeBackground()` server. A 30-second
`System.Threading.Timer` watchdog fires `Environment.Exit(1)` to prevent CI hangs.

## Build Requirements

- .NET 9 SDK
- Rust toolchain (for `conduit-capi`)
- On Linux: `libssl-dev` / `pkg-config` (for Rust TLS deps if any)
- macOS/Linux only — Windows build is skipped (cdylib linking needs cross-compile setup)

## File Layout

```
code/packages/csharp/conduit/
  CodingAdventures.Conduit.csproj
  Conduit.cs                          ← all source in one literate file
  tools/run-tests.sh
  tests/CodingAdventures.Conduit.Tests/
    CodingAdventures.Conduit.Tests.csproj
    ConduitTests.cs
  BUILD
  BUILD_windows
  README.md
  CHANGELOG.md
  required_capabilities.json

code/programs/csharp/conduit-hello/
  conduit-hello.csproj
  Program.cs
  tools/run-tests.sh
  BUILD
  BUILD_windows
  README.md
  CHANGELOG.md
  required_capabilities.json
```
