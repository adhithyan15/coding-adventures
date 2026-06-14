// SmokeTest.cs — end-to-end smoke tests for conduit-hello (WEB15)
//
// Spins up a server with the same routes as Program.cs, exercises each route
// via HttpClient, and asserts on status codes and response bodies.
//
// A 30-second watchdog timer fires Environment.Exit(1) if the test suite hangs,
// ensuring CI always completes within a finite time.

using System.Net;
using System.Text;
using System.Text.Json;
using CodingAdventures.Conduit;
using Xunit;

namespace ConduitHello.Tests;

// ── Server fixture ────────────────────────────────────────────────────────────

public sealed class HelloServerFixture : IDisposable
{
    public readonly HttpClient Http;
    public readonly string     BaseUrl;

    private readonly Server _server;
    private readonly System.Threading.Timer _watchdog;

    public HelloServerFixture()
    {
        var app = new Application();
        app.Set("app_name", "conduit-hello");
        app.Set("version",  "0.1.0");
        app.Set("env",      "test");

        var appName = app.GetSetting("app_name")!;
        var version = app.GetSetting("version")!;
        var env     = app.GetSetting("env")!;

        // After-hook mirrors Program.cs.
        app.After((req, resp) =>
            resp.WithHeader("x-served-by", $"{appName}/{version}")
                .WithHeader("x-env", env));

        app.Get("/", req => Response.Html($"<h1>{appName}</h1>"));

        app.Get("/health", req =>
            Response.Json(JsonSerializer.Serialize(new { status = "ok", name = appName, version, env })));

        app.Get("/api/greet/:name", req => {
            var name = req.Param("name") ?? "stranger";
            return Response.Json(JsonSerializer.Serialize(new { greeting = $"Hello, {name}!" }));
        });

        app.Get("/api/search", req => {
            var q = req.Query("q") ?? "";
            return Response.Json(JsonSerializer.Serialize(new { query = q }));
        });

        app.Post("/api/echo", req => {
            var ct = req.ContentType;
            ct = ct.StartsWith("application/json") ? "application/json"
               : ct.StartsWith("text/plain")        ? "text/plain; charset=utf-8"
               : "application/octet-stream";
            return Response.Respond(200, req.BodyString(), ("content-type", ct));
        });

        app.Get("/old-home", req => Response.Redirect("/"));

        app.Get("/tpot", req =>
            throw new HaltException(Response.Text("I'm a teapot", 418)));

        app.NotFound(req => {
            var b = JsonSerializer.Serialize(new { error = "not found", path = req.Path });
            return Response.Json(b, 404);
        });

        app.OnError(req => {
            Console.Error.WriteLine($"[smoke] error: {req.Error}");
            return Response.Json("{\"error\":\"internal server error\"}", 500);
        });

        _server = app.Bind("127.0.0.1", 0);
        _server.ServeBackground();

        BaseUrl = $"http://127.0.0.1:{_server.LocalPort}";
        Http    = new HttpClient { BaseAddress = new Uri(BaseUrl) };

        _watchdog = new System.Threading.Timer(
            _ => { Console.Error.WriteLine("[watchdog] smoke test timeout"); Environment.Exit(1); },
            null,
            TimeSpan.FromSeconds(30),
            System.Threading.Timeout.InfiniteTimeSpan);
    }

    public void Dispose()
    {
        _watchdog.Dispose();
        Http.Dispose();
        _server.Dispose();
    }
}

// ── Smoke tests ───────────────────────────────────────────────────────────────

[Collection("Smoke")]
public sealed class SmokeTests : IClassFixture<HelloServerFixture>
{
    private readonly HelloServerFixture _fx;
    public SmokeTests(HelloServerFixture fx) => _fx = fx;

    [Fact]
    public async Task Home_ReturnsHtml200()
    {
        var resp = await _fx.Http.GetAsync("/");
        Assert.Equal(HttpStatusCode.OK, resp.StatusCode);
        var body = await resp.Content.ReadAsStringAsync();
        Assert.Contains("conduit-hello", body);
    }

    [Fact]
    public async Task Health_ReturnsJsonWithStatusOk()
    {
        var resp = await _fx.Http.GetAsync("/health");
        Assert.Equal(HttpStatusCode.OK, resp.StatusCode);
        using var doc = JsonDocument.Parse(await resp.Content.ReadAsStringAsync());
        Assert.Equal("ok", doc.RootElement.GetProperty("status").GetString());
    }

    [Fact]
    public async Task GreetRoute_InjectsNameFromParam()
    {
        var resp = await _fx.Http.GetAsync("/api/greet/Alice");
        Assert.Equal(HttpStatusCode.OK, resp.StatusCode);
        using var doc = JsonDocument.Parse(await resp.Content.ReadAsStringAsync());
        Assert.Equal("Hello, Alice!", doc.RootElement.GetProperty("greeting").GetString());
    }

    [Fact]
    public async Task SearchRoute_QueryParam()
    {
        var resp = await _fx.Http.GetAsync("/api/search?q=conduit");
        Assert.Equal(HttpStatusCode.OK, resp.StatusCode);
        using var doc = JsonDocument.Parse(await resp.Content.ReadAsStringAsync());
        Assert.Equal("conduit", doc.RootElement.GetProperty("query").GetString());
    }

    [Fact]
    public async Task EchoRoute_ReflectsJsonBody()
    {
        var payload = new StringContent("{\"key\":\"val\"}", Encoding.UTF8, "application/json");
        var resp    = await _fx.Http.PostAsync("/api/echo", payload);
        Assert.Equal(HttpStatusCode.OK, resp.StatusCode);
        var body = await resp.Content.ReadAsStringAsync();
        Assert.Contains("key", body);
    }

    [Fact]
    public async Task Redirect_Returns302()
    {
        var handler = new HttpClientHandler { AllowAutoRedirect = false };
        using var client = new HttpClient(handler) { BaseAddress = new Uri(_fx.BaseUrl) };
        var resp = await client.GetAsync("/old-home");
        Assert.Equal(HttpStatusCode.Redirect, resp.StatusCode);
    }

    [Fact]
    public async Task HaltException_Returns418()
    {
        var resp = await _fx.Http.GetAsync("/tpot");
        Assert.Equal(418, (int)resp.StatusCode);
    }

    [Fact]
    public async Task NotFound_CustomHandler_Returns404()
    {
        var resp = await _fx.Http.GetAsync("/no-such-route-xyz");
        Assert.Equal(HttpStatusCode.NotFound, resp.StatusCode);
        using var doc = JsonDocument.Parse(await resp.Content.ReadAsStringAsync());
        Assert.Equal("not found", doc.RootElement.GetProperty("error").GetString());
    }

    [Fact]
    public async Task AfterHook_StampsXServedByOnAllRoutes()
    {
        var resp = await _fx.Http.GetAsync("/health");
        Assert.True(resp.Headers.TryGetValues("x-served-by", out var vals));
        Assert.Contains("conduit-hello/0.1.0", vals);
    }

    [Fact]
    public async Task AfterHook_StampsXEnv()
    {
        var resp = await _fx.Http.GetAsync("/");
        Assert.True(resp.Headers.TryGetValues("x-env", out var vals));
        Assert.Contains("test", vals);
    }
}
