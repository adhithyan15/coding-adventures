# CodingAdventures.Conduit.FSharp

A Sinatra/Express-style web framework for F# on .NET 9, built over the
`conduit-capi` Rust cdylib (WEB12). Part of the coding-adventures multi-language
conduit port series (WEB10–WEB18).

## Architecture

```
Program.fs  →  Application module  (functional builder, pipeline-compose)
               │  registers routes/hooks as F# lambdas
               ▼
            Trampolines module  (internal)
               │  Marshal.GetFunctionPointerForDelegate
               │  [UnmanagedFunctionPointer(Cdecl)]
               ▼
            Native module  (internal)
               │  [<DllImport("conduit_capi")>]
               ▼
            libconduit_capi.so / .dylib  (Rust cdylib — WEB12)
```

## Usage

```fsharp
open CodingAdventures.Conduit.FSharp

use server =
    Application.create()
    |> Application.set "app_name" "my-app"
    |> Application.set "version"  "1.0.0"
    |> Application.before (fun req ->
        if req.Path = "/admin" && req.Header "x-api-key" = None
        then Some (Response.json "{\"error\":\"unauthorized\"}" 401)
        else None)
    |> Application.after (fun _ resp ->
        resp |> Response.withHeader "x-powered-by" "conduit-fsharp")
    |> Application.get  "/" (fun _ -> Response.html "<h1>Hello!</h1>")
    |> Application.get  "/api/:name" (fun req ->
        let name = req.Param "name" |> Option.defaultValue "world"
        Response.json $"{{\"greeting\":\"Hello, {name}!\"}}")
    |> Application.post "/api/echo" (fun req -> Response.text (req.BodyString()))
    |> Application.notFound (fun req ->
        Response.json $"{{\"error\":\"not found\",\"path\":\"{req.Path}\"}}" 404)
    |> Application.onError (fun _ ->
        Response.json "{\"error\":\"internal server error\"}" 500)
    |> Application.bind "127.0.0.1" 3000us

printfn "Listening on port %d" server.LocalPort
server.Serve()  // blocks until Ctrl-C
```

## API Reference

### Response module

| Function | Description |
|---|---|
| `Response.html body ?status` | HTML response (status default 200) |
| `Response.json body ?status` | JSON response |
| `Response.text body ?status` | Plain-text response |
| `Response.respond status body headers` | Arbitrary status + headers |
| `Response.redirect location ?status` | 302 redirect (rejects CR/LF) |
| `Response.withHeader name value resp` | Append a header (returns new Response) |

### Application module

| Function | Description |
|---|---|
| `Application.create()` | Create a new builder |
| `Application.set key value app` | Store a named setting |
| `Application.getSetting key app` | Read a named setting (`string option`) |
| `Application.get pattern handler app` | Register a GET route |
| `Application.post pattern handler app` | Register a POST route |
| `Application.put/delete/patch ...` | Other verbs |
| `Application.before filter app` | Before-filter (`Request -> Response option`) |
| `Application.after hook app` | After-hook (`Request -> Response -> Response`) |
| `Application.notFound handler app` | Custom 404 handler |
| `Application.onError handler app` | Custom error handler |
| `Application.bind host port app` | Consume the builder; return a `Server` |

### Server type

| Member | Description |
|---|---|
| `server.LocalPort` | TCP port the server is listening on |
| `server.IsRunning` | True once the Tokio accept-loop is live |
| `server.Serve()` | Block current thread serving requests |
| `server.ServeBackground()` | Start on a background thread and return |
| `server.Stop()` | Signal the server to stop |
| `(server :> IDisposable).Dispose()` / `use` | Stop server and free native resources |

### HaltException

Throw from any handler to immediately short-circuit with a specific response:

```fsharp
app.Get("/secret", fun req ->
    raise (HaltException (Response.text "Forbidden" 403)))
```

## Running tests

```sh
sh tools/run-tests.sh
```

Requires: .NET 9 SDK, Rust toolchain (builds `conduit-capi` automatically).

## How it fits in the stack

`conduit-capi` (WEB12) is a Rust cdylib providing an HTTP server via Tokio/hyper.
This F# binding (WEB16) is one of seven language ports that use it:

| Port | Language |
|---|---|
| WEB12 | Rust cdylib (conduit-capi, the C ABI) |
| WEB13 | C++ |
| WEB14 | Go (cgo) |
| WEB15 | C# (P/Invoke) |
| WEB16 | **F# (P/Invoke)** ← you are here |
| WEB17 | Dart (dart:ffi) |
| WEB18 | Haskell (FFI) |
