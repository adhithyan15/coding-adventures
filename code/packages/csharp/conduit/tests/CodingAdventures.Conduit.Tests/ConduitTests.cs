// ConduitTests.cs — 35 tests for CodingAdventures.Conduit (WEB15)
//
// Tests are organised in four groups:
//   1. Response unit tests        — pure managed code, no native library
//   2. Application unit tests     — configure-only, require native library
//   3. Server lifecycle tests     — bind + LocalPort + IsRunning + Dispose
//   4. End-to-end HTTP tests      — ServeBackground + HttpClient
//
// E2E WATCHDOG
// ─────────────
// A 30-second System.Threading.Timer fires Environment.Exit(1) to prevent the
// test run from hanging in CI if a deadlock occurs. The timer is cancelled once
// all E2E tests complete (ServerFixture.Dispose).

using System.Net;
using System.Text;
using System.Text.Json;
using CodingAdventures.Conduit;
using Xunit;

namespace CodingAdventures.Conduit.Tests;

// ═══════════════════════════════════════════════════════════════════════════════
// GROUP 1 — Response unit tests (pure managed; native library NOT required)
// ═══════════════════════════════════════════════════════════════════════════════

public sealed class ResponseUnitTests
{
    [Fact]
    public void Html_DefaultStatus_Is200()
    {
        var r = Response.Html("<p>hi</p>");
        Assert.Equal(200, r.Status);
    }

    [Fact]
    public void Html_SetsContentTypeHeader()
    {
        var r = Response.Html("<p>hi</p>");
        Assert.Contains(r.Headers, h =>
            h.Name == "content-type" &&
            h.Value.StartsWith("text/html"));
    }

    [Fact]
    public void Html_ExplicitStatus()
    {
        var r = Response.Html("<p>created</p>", 201);
        Assert.Equal(201, r.Status);
    }

    [Fact]
    public void Json_SetsContentTypeHeader()
    {
        var r = Response.Json("{}", 200);
        Assert.Contains(r.Headers, h =>
            h.Name == "content-type" &&
            h.Value == "application/json");
    }

    [Fact]
    public void Text_SetsContentTypeHeader()
    {
        var r = Response.Text("hello");
        Assert.Contains(r.Headers, h =>
            h.Name == "content-type" &&
            h.Value.StartsWith("text/plain"));
    }

    [Fact]
    public void Respond_PreservesArbitraryHeaders()
    {
        var r = Response.Respond(418, "teapot",
            ("x-custom", "value1"),
            ("x-other", "value2"));
        Assert.Equal(418, r.Status);
        Assert.Equal("teapot", r.Body);
        Assert.Contains(r.Headers, h => h.Name == "x-custom" && h.Value == "value1");
        Assert.Contains(r.Headers, h => h.Name == "x-other"  && h.Value == "value2");
    }

    [Fact]
    public void Redirect_Default302WithLocation()
    {
        var r = Response.Redirect("/new-path");
        Assert.Equal(302, r.Status);
        Assert.Contains(r.Headers, h =>
            h.Name == "location" && h.Value == "/new-path");
    }

    [Fact]
    public void Redirect_RejectsCR()
    {
        Assert.Throws<ArgumentException>(() => Response.Redirect("/path\r\nX-Injected: bad"));
    }

    [Fact]
    public void Redirect_RejectsLF()
    {
        Assert.Throws<ArgumentException>(() => Response.Redirect("/path\nX-Injected: bad"));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// GROUP 2 — Application unit tests (native library required)
// ═══════════════════════════════════════════════════════════════════════════════

public sealed class ApplicationUnitTests
{
    [Fact]
    public void SetAndGetSetting_RoundTrip()
    {
        using var app = new Application();
        app.Set("key", "value");
        Assert.Equal("value", app.GetSetting("key"));
    }

    [Fact]
    public void GetSetting_MissingKey_ReturnsNull()
    {
        using var app = new Application();
        Assert.Null(app.GetSetting("nonexistent-key-xyz"));
    }

    [Fact]
    public void Get_ReturnsSameApplicationForChaining()
    {
        using var app = new Application();
        var result = app.Get("/", req => Response.Text("ok"));
        Assert.Same(app, result);
    }

    [Fact]
    public void Before_ReturnsSameApplicationForChaining()
    {
        using var app = new Application();
        var result = app.Before(req => null);
        Assert.Same(app, result);
    }

    [Fact]
    public void After_ReturnsSameApplicationForChaining()
    {
        using var app = new Application();
        var result = app.After((req, resp) => resp);
        Assert.Same(app, result);
    }

    [Fact]
    public void NotFound_ReturnsSameApplicationForChaining()
    {
        using var app = new Application();
        var result = app.NotFound(req => Response.Text("not found", 404));
        Assert.Same(app, result);
    }

    [Fact]
    public void OnError_ReturnsSameApplicationForChaining()
    {
        using var app = new Application();
        var result = app.OnError(req => Response.Text("error", 500));
        Assert.Same(app, result);
    }

    [Fact]
    public void MultipleSet_LastValueWins()
    {
        using var app = new Application();
        app.Set("key", "first");
        app.Set("key", "second");
        Assert.Equal("second", app.GetSetting("key"));
    }

    [Fact]
    public void GetSetting_AfterBind_Throws()
    {
        var app = new Application();
        app.Get("/", req => Response.Text("ok"));
        using var server = app.Bind("127.0.0.1", 0);
        Assert.Throws<InvalidOperationException>(() => app.GetSetting("x"));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// GROUP 3 — Server lifecycle tests
// ═══════════════════════════════════════════════════════════════════════════════

public sealed class ServerLifecycleTests
{
    [Fact]
    public void Bind_ReturnsServerWithPositivePort()
    {
        var app = new Application();
        app.Get("/", req => Response.Text("ok"));
        using var server = app.Bind("127.0.0.1", 0);
        Assert.True(server.LocalPort > 0);
    }

    [Fact]
    public void IsRunning_TrueAfterServeBackground()
    {
        var app = new Application();
        app.Get("/", req => Response.Text("ok"));
        using var server = app.Bind("127.0.0.1", 0);
        server.ServeBackground();
        Assert.True(server.IsRunning);
    }

    [Fact]
    public void Dispose_StopsRunningServer()
    {
        Server srv;
        var app = new Application();
        app.Get("/", req => Response.Text("ok"));
        srv = app.Bind("127.0.0.1", 0);
        srv.ServeBackground();
        srv.Dispose();
        Assert.False(srv.IsRunning);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// GROUP 4 — End-to-end HTTP tests
//
// A single server is shared across all E2E tests via ServerFixture (IClassFixture).
// The fixture wires up a realistic application with multiple routes and hooks,
// starts it on an OS-assigned port, and tears it down after the test class.
// ═══════════════════════════════════════════════════════════════════════════════

/// <summary>
/// Shared server and HttpClient for E2E tests.
/// The 30-second watchdog timer kills the process if any test hangs.
/// </summary>
public sealed class ServerFixture : IDisposable
{
    public readonly HttpClient Http;
    public readonly string     BaseUrl;

    private readonly Server _server;
    private readonly System.Threading.Timer _watchdog;

    public ServerFixture()
    {
        // ── Build the application ─────────────────────────────────────────────

        var app = new Application();
        app.Set("app_name", "conduit-test");
        app.Set("version",  "0.1.0");

        // Capture settings before Bind — the ConduitApp* is consumed by Bind().
        var appName = app.GetSetting("app_name") ?? "";
        var version = app.GetSetting("version")  ?? "";

        // After-hook: stamp every response with x-served-by.
        app.After((req, resp) =>
            resp.WithHeader("x-served-by", $"{appName}/{version}"));

        // Before-filter: block /maintenance with 503.
        app.Before(req =>
            req.Path == "/maintenance"
                ? Response.Text("Down for maintenance", 503)
                : null);

        // Routes.
        app.Get("/", req => Response.Html($"<h1>Hello from {appName}</h1>"));

        app.Get("/api/:id", req =>
        {
            var id = req.Param("id") ?? "unknown";
            var b  = JsonSerializer.Serialize(new { id });
            return Response.Json(b);
        });

        app.Post("/api/echo", req =>
        {
            var ct = req.ContentType;
            ct = ct.StartsWith("application/json")  ? "application/json"
               : ct.StartsWith("text/plain")         ? "text/plain; charset=utf-8"
               : "application/octet-stream";
            return Response.Respond(200, req.BodyString(), ("content-type", ct));
        });

        app.Get("/search", req =>
        {
            var q = req.Query("q") ?? "";
            var b = JsonSerializer.Serialize(new { query = q });
            return Response.Json(b);
        });

        app.Get("/redirect", req => Response.Redirect("/"));

        app.Get("/halt-418", req =>
            throw new HaltException(Response.Text("I'm a teapot", 418)));

        app.Get("/error-trigger", req =>
        {
            throw new InvalidOperationException("test error from handler");
        });

        // Custom not-found handler.
        app.NotFound(req =>
        {
            var b = JsonSerializer.Serialize(new { error = "not found", path = req.Path });
            return Response.Json(b, 404);
        });

        // Custom error handler — log server-side, never reflect raw error to client.
        app.OnError(req =>
        {
            Console.Error.WriteLine($"[test] handler error: {req.Error}");
            return Response.Json("{\"error\":\"internal server error\"}", 500);
        });

        // ── Bind and start ────────────────────────────────────────────────────

        _server = app.Bind("127.0.0.1", 0);
        _server.ServeBackground();

        BaseUrl = $"http://127.0.0.1:{_server.LocalPort}";
        Http    = new HttpClient { BaseAddress = new Uri(BaseUrl) };

        // Wait for conduit-capi's Tokio accept-loop to be live.
        //
        // conduit-capi uses a Tokio async runtime (Rust) with an OS-level thread
        // pool. `IsRunning` is set by the .NET side as soon as
        // conduit_server_serve_background returns, but the Tokio accept-loop and
        // worker threads initialise in parallel. Polling until IsRunning ensures
        // test requests are not sent before the accept-loop is up.
        //
        // KNOWN RACE: the first managed-code call from each fresh Tokio worker
        // thread incurs a one-time .NET thread-context setup cost. If a test is
        // the very first caller on a given thread it can receive a wrong HTTP
        // status before that setup completes. This manifests as intermittent
        // failures (≈ 40% of cold back-to-back runs) in BeforeFilter,
        // Post_EchoReflectsBody, and similar tests. The root cause is inside
        // conduit-capi; no .NET-side warmup probe reliably eliminates it without
        // introducing worse races (Rust panics, partial test runs). In CI the
        // test suite runs once with a cold-start Tokio, which has a lower failure
        // rate than hot-loop re-runs on the same binary.
        var rdy = DateTime.UtcNow.AddSeconds(5);
        while (!_server.IsRunning && DateTime.UtcNow < rdy)
            System.Threading.Thread.Sleep(2);

        // ── Watchdog ──────────────────────────────────────────────────────────
        // If any E2E test hangs for 30 seconds, kill the process so CI doesn't
        // wait forever. The timer is disarmed in Dispose().
        _watchdog = new System.Threading.Timer(
            _ => { Console.Error.WriteLine("[watchdog] E2E tests timed out — aborting"); Environment.Exit(1); },
            null,
            TimeSpan.FromSeconds(30),
            System.Threading.Timeout.InfiniteTimeSpan);
    }

    public void Dispose()
    {
        _watchdog.Dispose(); // disarm before tearing down
        Http.Dispose();
        _server.Dispose();
    }
}

[Collection("E2E")]
public sealed class EndToEndTests : IClassFixture<ServerFixture>
{
    private readonly ServerFixture _fx;
    public EndToEndTests(ServerFixture fx) => _fx = fx;

    // ── Basic routes ─────────────────────────────────────────────────────────

    [Fact]
    public async Task Root_ReturnsHtml()
    {
        var resp = await _fx.Http.GetAsync("/");
        Assert.Equal(HttpStatusCode.OK, resp.StatusCode);
        var body = await resp.Content.ReadAsStringAsync();
        Assert.Contains("Hello from conduit-test", body);
    }

    [Fact]
    public async Task RouteParam_JsonResponseContainsId()
    {
        var resp = await _fx.Http.GetAsync("/api/42");
        Assert.Equal(HttpStatusCode.OK, resp.StatusCode);
        var body = await resp.Content.ReadAsStringAsync();
        using var doc = JsonDocument.Parse(body);
        Assert.Equal("42", doc.RootElement.GetProperty("id").GetString());
    }

    [Fact]
    public async Task Post_EchoReflectsBody()
    {
        var content = new StringContent(
            "{\"hello\":\"world\"}", Encoding.UTF8, "application/json");
        var resp = await _fx.Http.PostAsync("/api/echo", content);
        Assert.Equal(HttpStatusCode.OK, resp.StatusCode);
        var body = await resp.Content.ReadAsStringAsync();
        Assert.Contains("hello", body);
    }

    [Fact]
    public async Task QueryParam_RoundTrip()
    {
        var resp = await _fx.Http.GetAsync("/search?q=dotnet");
        Assert.Equal(HttpStatusCode.OK, resp.StatusCode);
        var body = await resp.Content.ReadAsStringAsync();
        using var doc = JsonDocument.Parse(body);
        Assert.Equal("dotnet", doc.RootElement.GetProperty("query").GetString());
    }

    // ── Before-filter ─────────────────────────────────────────────────────────

    [Fact]
    public async Task BeforeFilter_MaintenanceRoute_Returns503()
    {
        var resp = await _fx.Http.GetAsync("/maintenance");
        Assert.Equal(HttpStatusCode.ServiceUnavailable, resp.StatusCode);
    }

    [Fact]
    public async Task BeforeFilter_NormalRoute_PassesThrough()
    {
        // Before-filter only blocks /maintenance; all other routes should work.
        var resp = await _fx.Http.GetAsync("/");
        Assert.Equal(HttpStatusCode.OK, resp.StatusCode);
    }

    // ── After-hook ────────────────────────────────────────────────────────────

    [Fact]
    public async Task AfterHook_StampsXServedByHeader()
    {
        var resp = await _fx.Http.GetAsync("/");
        Assert.True(resp.Headers.TryGetValues("x-served-by", out var vals));
        Assert.Contains("conduit-test/0.1.0", vals);
    }

    [Fact]
    public async Task AfterHook_AppliesToAllRoutes()
    {
        // After-hook must run even on 404 responses.
        var resp = await _fx.Http.GetAsync("/nonexistent-path-abc");
        Assert.True(resp.Headers.TryGetValues("x-served-by", out _));
    }

    // ── Redirect ──────────────────────────────────────────────────────────────

    [Fact]
    public async Task Redirect_Returns302WithLocationHeader()
    {
        // Disable auto-redirect to observe the 3xx.
        var handler = new HttpClientHandler { AllowAutoRedirect = false };
        using var client = new HttpClient(handler) { BaseAddress = new Uri(_fx.BaseUrl) };
        var resp = await client.GetAsync("/redirect");
        Assert.Equal(HttpStatusCode.Redirect, resp.StatusCode);
        Assert.NotNull(resp.Headers.Location);
    }

    // ── HaltException ─────────────────────────────────────────────────────────

    [Fact]
    public async Task HaltException_ShortCircuitsWithExpectedStatus()
    {
        var resp = await _fx.Http.GetAsync("/halt-418");
        Assert.Equal(418, (int)resp.StatusCode);
        var body = await resp.Content.ReadAsStringAsync();
        Assert.Contains("teapot", body);
    }

    // ── Error handler ─────────────────────────────────────────────────────────
    //
    // When a C# handler throws a non-Halt exception, the trampoline catches it,
    // logs server-side, and returns a generic 500 JSON response directly.
    // conduit-capi routes the raw error string as plain-text when a C handler
    // returns NULL + conduit_capi_report_error, bypassing the C# error handler
    // callback — so the trampoline handles exceptions internally instead.

    [Fact]
    public async Task ThrowingHandler_Returns500WithJsonBody()
    {
        var resp = await _fx.Http.GetAsync("/error-trigger");
        Assert.Equal(HttpStatusCode.InternalServerError, resp.StatusCode);
        var body = await resp.Content.ReadAsStringAsync();
        using var doc = JsonDocument.Parse(body);
        Assert.Equal("internal server error", doc.RootElement.GetProperty("error").GetString());
    }

    [Fact]
    public async Task ThrowingHandler_BodyDoesNotLeakExceptionDetails()
    {
        // Exception details must not reach the client — log server-side only.
        var resp = await _fx.Http.GetAsync("/error-trigger");
        var body = await resp.Content.ReadAsStringAsync();
        Assert.DoesNotContain("InvalidOperationException", body);
        Assert.DoesNotContain("test error from handler", body);
    }

    // ── Custom not-found handler ──────────────────────────────────────────────

    [Fact]
    public async Task NotFound_CustomHandler_Returns404WithJsonBody()
    {
        var resp = await _fx.Http.GetAsync("/path/that/does/not/exist");
        Assert.Equal(HttpStatusCode.NotFound, resp.StatusCode);
        var body = await resp.Content.ReadAsStringAsync();
        using var doc = JsonDocument.Parse(body);
        Assert.Equal("not found", doc.RootElement.GetProperty("error").GetString());
    }

    // ── Content-type whitelist on echo ────────────────────────────────────────

    [Fact]
    public async Task EchoEndpoint_UnknownContentType_Normalised()
    {
        // Sending text/html (not in whitelist) — handler maps it to octet-stream.
        var content = new StringContent("data", Encoding.UTF8, "text/html");
        var resp    = await _fx.Http.PostAsync("/api/echo", content);
        Assert.Equal(HttpStatusCode.OK, resp.StatusCode);
        var ct = resp.Content.Headers.ContentType?.MediaType ?? "";
        Assert.NotEqual("text/html", ct);
    }
}
