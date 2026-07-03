# conduit-hello

A demo .NET 9 console application showing idiomatic usage of
[CodingAdventures.Conduit](../../packages/csharp/conduit/README.md) (WEB15).

## Routes

| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | HTML home page |
| GET | `/health` | JSON health check |
| GET | `/api/greet/:name` | JSON greeting with route parameter |
| GET | `/api/search?q=…` | JSON search with query string |
| POST | `/api/echo` | Echo request body (safe content types only) |
| GET | `/old-home` | Redirect to `/` |
| GET | `/tpot` | 418 via HaltException |

All routes get `x-served-by` and `x-env` headers stamped by an after-hook.

## Run

```sh
# Build the Rust native library first
cd ../../packages/rust/conduit-capi && cargo build --release

# Start the server
cd ../../programs/csharp/conduit-hello
CONDUIT_CAPI_PATH=../../packages/rust/target/release/libconduit_capi.dylib \
  dotnet run

# Optional overrides
HOST=0.0.0.0 PORT=8080 APP_ENV=production dotnet run
```

## Smoke tests

```sh
sh tools/run-tests.sh
```

10 smoke tests exercise each route and verify after-hook header stamping.

## Key patterns demonstrated

**Capture settings before Bind():**
```csharp
var app = new Application();
app.Set("app_name", "conduit-hello");
var appName = app.GetSetting("app_name")!;  // ← read here
app.Get("/", req => Response.Html($"<h1>{appName}</h1>"));  // ← capture string
using var server = app.Bind();  // ← ConduitApp* consumed here
```

**Before-filter returns null to continue:**
```csharp
app.Before(req => req.Path == "/health" ? null : CheckAuth(req));
```

**After-hook adds a header:**
```csharp
app.After((req, resp) => resp.WithHeader("x-served-by", $"{appName}/{version}"));
```

**Structured JSON (no string interpolation into JSON):**
```csharp
return Response.Json(JsonSerializer.Serialize(new { greeting = $"Hello, {name}!" }));
```
