# WEB16 — F# Conduit Binding

## Purpose

`CodingAdventures.Conduit.FSharp` is a Sinatra/Express-style web framework for
F# on .NET 9, built over the `conduit-capi` Rust cdylib (WEB12). It wraps the
same C ABI used by Go (WEB14) and C# (WEB15), providing a functional, pipe-friendly
API idiomatic to F#.

## Scope

- Package: `code/packages/fsharp/conduit/`
- Demo program: `code/programs/fsharp/conduit-hello/`
- Tests: ≥ 35 tests, ≥ 80% line coverage
- Languages: F# 8 / .NET 9

## Architecture

```
Program.fs          ─► Application module (F# builder)
                        │  pipeline-compose routes/hooks/filters
                        ▼
                    Trampolines module (internal)
                        │  Marshal.GetFunctionPointerForDelegate
                        │  [UnmanagedFunctionPointer(Cdecl)]
                        ▼
                    Native module (internal)
                        │  [<DllImport("conduit_capi")>]
                        ▼
                    libconduit_capi.so / .dylib  (Rust cdylib — WEB12)
```

The binding reuses **conduit-capi** verbatim — no new Rust code.

## Delegate Lifetime

F# closures registered as route handlers, before-filters, and after-hooks are
heap-allocated objects. To survive GC while Rust holds function-pointer references
to them, each is wrapped in a `GCHandle.Alloc(fn, GCHandleType.Normal)` strong
root. The handle's `IntPtr` is passed as the opaque `ctx` to conduit-capi.

When conduit-capi calls `ctx_free` (on server teardown), the trampoline calls
`GCHandle.FromIntPtr(ctx).Free()`, releasing the root.

```
F# managed heap                     C native heap
┌──────────────────┐               ┌─────────────────────────┐
│  F# lambda (fn)  │◄─ GCHandle ──►│ ctx (opaque nativeint)  │
└──────────────────┘  (strong root) └─────────────────────────┘
        ▲                                       │
        └───────────────────────────────────────┘
              trampoline recovers via
         GCHandle.FromIntPtr(ctx).Target :?> _
```

## Trampoline Strategy

F# does not have idiomatic access to C# 9's `delegate*` syntax. Instead we use
the classic .NET pattern:

1. Declare a delegate type with `[<UnmanagedFunctionPointer(CallingConvention.Cdecl)>]`.
2. Create a static module-level delegate instance (prevents GC).
3. Obtain a native function pointer via `Marshal.GetFunctionPointerForDelegate`.

This matches the ABI that conduit-capi expects (cdecl on x86-64 / arm64-darwin).

## F# API Surface

```fsharp
// ── Response ──────────────────────────────────────────────────────────────────
type Response = {
    Status  : int
    Body    : string
    Headers : (string * string) list
}

module Response =
    // F# module-level let bindings cannot have optional parameters (FS0718).
    // Use |> Response.withStatus to override the default (200 / 302).
    val html       : body:string -> Response                               // 200
    val json       : body:string -> Response                               // 200
    val text       : body:string -> Response                               // 200
    val respond    : status:int -> body:string -> headers:(string * string) list -> Response
    val redirect   : location:string -> Response                           // 302
    val withHeader : name:string -> value:string -> Response -> Response
    val withStatus : status:int -> Response -> Response

// ── HaltException ─────────────────────────────────────────────────────────────
exception HaltException of Response          // throw to short-circuit a handler

// ── Request ───────────────────────────────────────────────────────────────────
type Request
    member Method      : string
    member Path        : string
    member QueryString : string
    member ContentType : string
    member RemoteAddr  : string
    member Error       : string
    member Param       : name:string -> string option
    member Query       : name:string -> string option
    member Header      : name:string -> string option
    member Body        : unit -> byte array
    member BodyString  : unit -> string

// ── Application (opaque builder) ─────────────────────────────────────────────
type Application

module Application =
    val create     : unit -> Application
    val set        : key:string -> value:string -> Application -> Application
    val getSetting : key:string -> Application -> string option
    val get        : pattern:string -> handler:(Request -> Response) -> Application -> Application
    val post       : pattern:string -> handler:(Request -> Response) -> Application -> Application
    val put        : pattern:string -> handler:(Request -> Response) -> Application -> Application
    val delete     : pattern:string -> handler:(Request -> Response) -> Application -> Application
    val patch      : pattern:string -> handler:(Request -> Response) -> Application -> Application
    val route      : method:string -> pattern:string -> handler:(Request -> Response) -> Application -> Application
    val before     : filter:(Request -> Response option) -> Application -> Application
    val after      : hook:(Request -> Response -> Response) -> Application -> Application
    val notFound   : handler:(Request -> Response) -> Application -> Application
    val onError    : handler:(Request -> Response) -> Application -> Application
    val bind       : host:string -> port:uint16 -> Application -> Server

// ── Server ────────────────────────────────────────────────────────────────────
type Server
    member LocalPort       : uint16
    member IsRunning       : bool
    member Serve           : unit -> unit
    member ServeBackground : unit -> unit
    member Stop            : unit -> unit
    interface IDisposable
```

## Security Requirements

All security properties from WEB15 (C# port) carry over:

| Property | Implementation |
|---|---|
| No TOCTOU in lib load | Load `CONDUIT_CAPI_PATH` directly — no `File.Exists` pre-check |
| Null ctx guards | All trampolines return safe 500 / pass-through on `ctx = 0n` |
| Bounds check before allocation | `nuint→int` cast guarded in `Request.Body()` and `Response.fromNative` |
| Sanitised error in bind failure | Public exception message is generic; raw Rust error → stderr only |
| Inverted auth bypass | conduit-hello enforces by default; bypass must be explicitly opted in |
| HTML-encode in template | All server-controlled values encoded with `WebUtility.HtmlEncode` |
| Status code range validation | `[100, 999]` check before `uint16` cast in `toNative` |

## Tests

| Group | Count | Description |
|---|---|---|
| 1 — Response unit | 8 | Pure managed code; no native lib |
| 2 — Application unit | 9 | Configure-only; requires native lib |
| 3 — Server lifecycle | 5 | bind / LocalPort / IsRunning / Dispose |
| 4 — End-to-end HTTP | 15 | ServeBackground + HttpClient |
| **Total** | **37** | ≥ 37 tests, ≥ 80% line coverage |

A 30-second E2E watchdog fires `Environment.Exit(1)` if tests hang (CI safety net).

## Known Race

conduit-capi's Tokio worker threads need one-time .NET managed-context setup on
their first P/Invoke callback. This causes intermittent E2E failures (~40% in
back-to-back dev runs). Root cause is inside conduit-capi. Documented in the
`ServerFixture` comment; not a regression of this F# binding.

## Files

```
code/specs/WEB16-conduit-fsharp.md                   (this file)
code/packages/fsharp/conduit/
  .gitignore
  BUILD
  BUILD_windows
  CHANGELOG.md
  CodingAdventures.Conduit.fsproj
  Conduit.fs
  README.md
  required_capabilities.json
  tools/run-tests.sh
  tests/CodingAdventures.Conduit.Tests/
    CodingAdventures.Conduit.Tests.fsproj
    ConduitTests.fs
code/programs/fsharp/conduit-hello/
  .gitignore
  BUILD
  BUILD_windows
  CHANGELOG.md
  Program.fs
  README.md
  conduit-hello.fsproj
  required_capabilities.json
  tools/run-tests.sh
  tests/ConduitHello.Smoke/
    ConduitHello.Smoke.fsproj
    SmokeTest.fs
```
