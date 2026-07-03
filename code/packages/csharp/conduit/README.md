# CodingAdventures.Conduit

A .NET 9 P/Invoke binding for the **conduit-capi** C ABI — a Sinatra/Express-style
web framework built on Rust's `web-core` engine.

## Stack position

```
Program.cs (your app)
    │
CodingAdventures.Conduit  ← this package
    │  P/Invoke / [DllImport]
    ▼
libconduit_capi.so/.dylib  (Rust, WEB12)
    │
web-core  (Rust engine, WEB08)
```

## Quick start

```csharp
using CodingAdventures.Conduit;

var app = new Application();
app.Set("title", "MyApp");
var title = app.GetSetting("title")!;   // capture before Bind()

app.Get("/", req => Response.Html($"<h1>Welcome to {title}</h1>"));

app.Get("/api/user/:id", req => {
    var id = req.Param("id");
    return Response.Json(System.Text.Json.JsonSerializer.Serialize(new { id }));
});

app.Before(req =>
    req.Header("x-api-key") == "secret"
        ? null                               // continue
        : Response.Text("Unauthorized", 401));

app.After((req, resp) => resp.WithHeader("x-powered-by", "conduit"));

app.NotFound(req => Response.Json("{\"error\":\"not found\"}", 404));

app.OnError(req => {
    Console.Error.WriteLine($"Error: {req.Error}");
    return Response.Json("{\"error\":\"internal server error\"}", 500);
});

using var server = app.Bind("0.0.0.0", 8080);
server.Serve();
```

## API reference

### Response

| Method | Description |
|--------|-------------|
| `Response.Html(body, status=200)` | `text/html; charset=utf-8` response |
| `Response.Json(body, status=200)` | `application/json` response |
| `Response.Text(body, status=200)` | `text/plain; charset=utf-8` response |
| `Response.Respond(status, body, ...headers)` | Arbitrary status + headers |
| `Response.Redirect(location, status=302)` | Location redirect (throws on CR/LF) |
| `resp.WithHeader(name, value)` | Fluent header addition |

### Request

| Property / Method | Description |
|------------------|-------------|
| `req.Method` | `"GET"`, `"POST"`, … |
| `req.Path` | `/api/users/42` |
| `req.QueryString` | Raw query string without `?` |
| `req.ContentType` | `Content-Type` header value |
| `req.RemoteAddr` | `"127.0.0.1:54321"` |
| `req.Error` | Error message (non-empty only in `OnError`) |
| `req.Param("name")` | Route parameter or `null` |
| `req.Query("key")` | Query string value or `null` |
| `req.Header("name")` | Case-insensitive header lookup or `null` |
| `req.Body()` | Raw body as `byte[]` |
| `req.BodyString()` | Body decoded as UTF-8 string |

### Application (builder)

```csharp
var app = new Application();
app.Set("key", "value")           // store a setting
app.GetSetting("key")             // retrieve (before Bind only)
app.Get("/path", handler)
app.Post("/path", handler)
app.Put("/path", handler)
app.Delete("/path", handler)
app.Patch("/path", handler)
app.Route("CUSTOM", "/path", handler)
app.Before(filter)                // null = continue; Response = halt
app.After(hook)                   // receive + transform current response
app.NotFound(handler)
app.OnError(handler)
using var server = app.Bind(host, port);  // consumes the Application
```

### HaltException

Throw from anywhere inside a handler to immediately short-circuit:

```csharp
app.Before(req => {
    if (!IsValid(req))
        throw new HaltException(Response.Text("Bad Request", 400));
    return null;
});
```

### Server

```csharp
server.Serve()            // blocks until Stop()
server.ServeBackground()  // returns immediately; runs in OS thread
server.Stop()
server.LocalPort          // ushort — actual bound port
server.IsRunning          // bool
```

## Delegate lifetime management

Every C# lambda passed to `app.Get(...)`, `app.Before(...)`, etc. is stored in a
`GCHandle` (allocated, not pinned). The GC cannot collect the lambda as long as the
handle is alive. When conduit-capi frees the app/server, it calls our `ctx_free`
trampoline which calls `GCHandle.Free()`, releasing the strong root.

This is the exact analogue of Go's `cgo.Handle` pattern used in WEB14.

## Build requirements

- .NET 9 SDK
- Rust toolchain (`cargo`)

```sh
cd code/packages/csharp/conduit
sh tools/run-tests.sh
```

The script builds `conduit-capi` (Rust cdylib), sets `CONDUIT_CAPI_PATH`, then runs
`dotnet test` with coverage.

## Security properties

All header injection defence, status clamping (100–599), and UTF-8 validation are
handled by `conduit-capi` in Rust — trusted once, reused across all language ports.

The C# layer adds:
- Redirect location CR/LF guard (throws `ArgumentException`)
- Error message sanitisation in trampolines (strips control chars, caps at 512 bytes)
- Structured JSON via `System.Text.Json` (no string interpolation into responses)

## Test coverage

35 tests across four groups:

| Group | Count |
|-------|-------|
| Response factory (pure managed) | 9 |
| Application configuration | 9 |
| Server lifecycle | 3 |
| End-to-end HTTP | 14 |
