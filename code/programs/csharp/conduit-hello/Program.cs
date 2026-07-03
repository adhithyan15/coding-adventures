// conduit-hello — demonstration of CodingAdventures.Conduit (WEB15)
//
// This program shows the idiomatic C# usage pattern:
//   1. Create an Application and store settings.
//   2. Read settings BEFORE Bind() — the ConduitApp* is consumed on Bind().
//   3. Register routes using lambdas that capture read settings as local variables.
//   4. Register before-filters and after-hooks.
//   5. Bind to a port and call Serve() (blocks until Ctrl-C).
//
// Run:  CONDUIT_CAPI_PATH=<path-to-lib> dotnet run
// Test: sh tools/run-tests.sh

using System.Net;
using System.Text.Json;
using CodingAdventures.Conduit;

// ── Application setup ────────────────────────────────────────────────────────

var app = new Application();

// Store runtime configuration as named settings.
app.Set("app_name", "conduit-hello");
app.Set("version",  "0.1.0");
app.Set("env",      Environment.GetEnvironmentVariable("APP_ENV") ?? "development");

// IMPORTANT: read settings now — after Bind(), the ConduitApp* is gone.
var appName = app.GetSetting("app_name") ?? "conduit-hello";
var version = app.GetSetting("version")  ?? "0.1.0";
var env     = app.GetSetting("env")      ?? "development";

// ── Before-filter: simple API-key guard ──────────────────────────────────────
//
// Return null to pass through; return a Response to short-circuit.

app.Before(req => {
    // Let static assets and the health check pass without auth.
    if (req.Path == "/health" || req.Path == "/") return null;

    // Opt-in to bypass, not opt-in to enforcement. Enforce auth in all environments
    // except the explicitly-whitelisted "development" value. This way a misconfigured
    // deploy that forgets to set APP_ENV="production" is still protected.
    var key = req.Header("x-api-key");
    if (env != "development" && string.IsNullOrEmpty(key))
        return Response.Json("{\"error\":\"missing x-api-key header\"}", 401);

    return null;
});

// ── After-hook: stamp every response with server metadata ─────────────────────

app.After((req, resp) =>
    resp.WithHeader("x-served-by", $"{appName}/{version}")
        .WithHeader("x-env",        env));

// ── Routes ────────────────────────────────────────────────────────────────────

// Home page — demonstrates HTML response.
// HTML-encode all server-controlled values embedded in the template.
// Defence-in-depth: even operator-supplied env/version values must not
// be trusted to be free of HTML metacharacters.
var safeVersion = WebUtility.HtmlEncode(version);
var safeEnv     = WebUtility.HtmlEncode(env);
var safeAppName = WebUtility.HtmlEncode(appName);

app.Get("/", req =>
    Response.Html($"""
        <!doctype html>
        <html>
          <body>
            <h1>{safeAppName}</h1>
            <p>Version: {safeVersion} | Env: {safeEnv}</p>
            <ul>
              <li><a href="/health">/health</a></li>
              <li><a href="/api/greet/World">/api/greet/:name</a></li>
              <li><a href="/api/search?q=conduit">/api/search?q=…</a></li>
            </ul>
          </body>
        </html>
        """));

// Health check — used by load balancers / orchestration.
app.Get("/health", req =>
    Response.Json(JsonSerializer.Serialize(new
    {
        status  = "ok",
        name    = appName,
        version,
        env,
    })));

// Route parameter — /api/greet/:name
app.Get("/api/greet/:name", req => {
    var name = req.Param("name") ?? "stranger";
    // Use JsonSerializer for safety — never sprintf user input into JSON.
    return Response.Json(JsonSerializer.Serialize(new
    {
        greeting = $"Hello, {name}!",
        from     = appName,
    }));
});

// Query string — /api/search?q=…&limit=…
app.Get("/api/search", req => {
    var q     = req.Query("q")     ?? "";
    var limit = req.Query("limit") ?? "10";
    if (!int.TryParse(limit, out var n) || n < 1 || n > 100) n = 10;

    return Response.Json(JsonSerializer.Serialize(new
    {
        query = q,
        limit = n,
        // In a real app, results would come from a database.
        results = Array.Empty<string>(),
    }));
});

// Echo body — demonstrates POST request body access.
// Only mirrors safe content types to avoid content-sniffing attacks.
app.Post("/api/echo", req => {
    var ct = req.ContentType;
    ct = ct.StartsWith("application/json") ? "application/json"
       : ct.StartsWith("text/plain")        ? "text/plain; charset=utf-8"
       : "application/octet-stream";
    return Response.Respond(200, req.BodyString(), ("content-type", ct));
});

// Redirect — demonstrates 3xx responses.
app.Get("/old-home", req => Response.Redirect("/"));

// Halt — demonstrates non-local exit via HaltException.
app.Get("/tpot", req =>
    throw new HaltException(Response.Text("I'm a teapot", 418)));

// ── Error handling ────────────────────────────────────────────────────────────

app.NotFound(req => {
    var b = JsonSerializer.Serialize(new
    {
        error = "not found",
        path  = req.Path,
    });
    return Response.Json(b, 404);
});

app.OnError(req => {
    // Log the real error server-side; never expose it to clients.
    Console.Error.WriteLine($"[{appName}] handler error: {req.Error}");
    return Response.Json("{\"error\":\"internal server error\"}", 500);
});

// ── Bind and serve ────────────────────────────────────────────────────────────

var host = Environment.GetEnvironmentVariable("HOST") ?? "127.0.0.1";
var portStr = Environment.GetEnvironmentVariable("PORT") ?? "3000";
ushort.TryParse(portStr, out var port);

Console.WriteLine($"[{appName}] starting on {host}:{port} (env={env})");

using var server = app.Bind(host, port);
Console.WriteLine($"[{appName}] listening on port {server.LocalPort}");

server.Serve();
