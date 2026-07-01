// ConduitTests.fs — 37 tests for CodingAdventures.Conduit.FSharp (WEB16)
//
// Tests are organised in four groups:
//   1. Response unit tests        — pure managed code, no native library
//   2. Application unit tests     — configure-only, require native library
//   3. Server lifecycle tests     — bind + LocalPort + IsRunning + Dispose
//   4. End-to-end HTTP tests      — ServeBackground + HttpClient
//
// E2E WATCHDOG
// ─────────────
// A 30-second System.Threading.Timer fires Environment.Exit(1) to prevent
// the test run from hanging in CI if a deadlock occurs. The timer is cancelled
// once all E2E tests complete (ServerFixture.Dispose).

module CodingAdventures.Conduit.FSharp.Tests

open System
open System.Net
open System.Net.Http
open System.Text
open System.Text.Json
open CodingAdventures.Conduit.FSharp
open Xunit

// ═════════════════════════════════════════════════════════════════════════════
// GROUP 1 — Response unit tests (pure managed; native library NOT required)
// ═════════════════════════════════════════════════════════════════════════════

module ResponseUnitTests =

    [<Fact>]
    let ``html default status is 200`` () =
        let r = Response.html "<p>hi</p>"
        Assert.Equal(200, r.Status)

    [<Fact>]
    let ``html sets content-type header`` () =
        let r = Response.html "<p>hi</p>"
        Assert.Contains(r.Headers, fun (n, v) -> n = "content-type" && v.StartsWith "text/html")

    [<Fact>]
    let ``html with explicit status`` () =
        let r = Response.html "<p>created</p>" |> Response.withStatus 201
        Assert.Equal(201, r.Status)

    [<Fact>]
    let ``json sets content-type`` () =
        let r = Response.json "{}"
        Assert.Contains(r.Headers, fun (n, v) -> n = "content-type" && v = "application/json")

    [<Fact>]
    let ``text sets content-type`` () =
        let r = Response.text "hello"
        Assert.Contains(r.Headers, fun (n, v) -> n = "content-type" && v.StartsWith "text/plain")

    [<Fact>]
    let ``respond preserves status body and headers`` () =
        let r = Response.respond 418 "teapot" [("x-custom", "value1"); ("x-other", "value2")]
        Assert.Equal(418, r.Status)
        Assert.Equal("teapot", r.Body)
        Assert.Contains(r.Headers, fun (n, v) -> n = "x-custom" && v = "value1")
        Assert.Contains(r.Headers, fun (n, v) -> n = "x-other"  && v = "value2")

    [<Fact>]
    let ``redirect default 302 with location`` () =
        let r = Response.redirect "/new-path"
        Assert.Equal(302, r.Status)
        Assert.Contains(r.Headers, fun (n, v) -> n = "location" && v = "/new-path")

    [<Fact>]
    let ``redirect rejects CR`` () =
        Assert.Throws<ArgumentException>(fun () ->
            Response.redirect "/path\r\nX-Injected: bad" |> ignore) |> ignore

    [<Fact>]
    let ``redirect rejects LF`` () =
        Assert.Throws<ArgumentException>(fun () ->
            Response.redirect "/path\nX-Injected: bad" |> ignore) |> ignore

    [<Fact>]
    let ``withHeader appends header`` () =
        let r = Response.html "body" |> Response.withHeader "x-foo" "bar"
        Assert.Contains(r.Headers, fun (n, v) -> n = "x-foo" && v = "bar")
        // Original header still present
        Assert.Contains(r.Headers, fun (n, _) -> n = "content-type")

    [<Fact>]
    let ``status out of range throws`` () =
        Assert.Throws<ArgumentException>(fun () ->
            Response.html "body" |> Response.withStatus 99 |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () ->
            Response.json "{}" |> Response.withStatus 1000 |> ignore) |> ignore

// ═════════════════════════════════════════════════════════════════════════════
// GROUP 2 — Application unit tests (configure-only; requires native library)
// ═════════════════════════════════════════════════════════════════════════════

module ApplicationUnitTests =

    [<Fact>]
    let ``create returns an Application`` () =
        use app = Application.create()
        Assert.NotNull app

    [<Fact>]
    let ``set and getSetting round-trip`` () =
        use app = Application.create()
        let app2 = app |> Application.set "foo" "bar"
        Assert.Equal(Some "bar", Application.getSetting "foo" app2)

    [<Fact>]
    let ``getSetting returns None for missing key`` () =
        use app = Application.create()
        Assert.Equal(None, Application.getSetting "no-such-key" app)

    [<Fact>]
    let ``multiple settings are independent`` () =
        use app = Application.create()
        let app2 =
            app
            |> Application.set "a" "alpha"
            |> Application.set "b" "beta"
        Assert.Equal(Some "alpha", Application.getSetting "a" app2)
        Assert.Equal(Some "beta",  Application.getSetting "b" app2)

    [<Fact>]
    let ``get registration does not throw`` () =
        use app = Application.create()
        app |> Application.get "/" (fun _ -> Response.html "<h1>Hi</h1>") |> ignore

    [<Fact>]
    let ``post registration does not throw`` () =
        use app = Application.create()
        app |> Application.post "/api" (fun _ -> Response.json "{}") |> ignore

    [<Fact>]
    let ``before filter registration does not throw`` () =
        use app = Application.create()
        app |> Application.before (fun _ -> None) |> ignore

    [<Fact>]
    let ``after hook registration does not throw`` () =
        use app = Application.create()
        app |> Application.after (fun _ resp -> resp) |> ignore

    [<Fact>]
    let ``notFound and onError registration does not throw`` () =
        use app = Application.create()
        app
        |> Application.notFound (fun req -> Response.json (sprintf "{\"error\":\"not found\",\"path\":\"%s\"}" req.Path) |> Response.withStatus 404)
        |> Application.onError (fun _ -> Response.json "{\"error\":\"internal server error\"}" |> Response.withStatus 500)
        |> ignore

// ═════════════════════════════════════════════════════════════════════════════
// GROUP 3 — Server lifecycle tests
// ═════════════════════════════════════════════════════════════════════════════

module ServerLifecycleTests =

    [<Fact>]
    let ``isRunning true after serveBackground`` () =
        use server =
            Application.create()
            |> Application.get "/" (fun _ -> Response.html "<h1>Hi</h1>")
            |> Application.bind "127.0.0.1" 0us
        server.ServeBackground()
        let deadline = DateTime.UtcNow.AddSeconds 5.0
        while not server.IsRunning && DateTime.UtcNow < deadline do
            System.Threading.Thread.Sleep 2
        Assert.True server.IsRunning

    [<Fact>]
    let ``localPort is non-zero after bind`` () =
        use server =
            Application.create()
            |> Application.get "/" (fun _ -> Response.text "ok")
            |> Application.bind "127.0.0.1" 0us
        Assert.NotEqual(0us, server.LocalPort)

    [<Fact>]
    let ``isRunning false after dispose`` () =
        let server =
            Application.create()
            |> Application.get "/" (fun _ -> Response.text "ok")
            |> Application.bind "127.0.0.1" 0us
        server.ServeBackground()
        let deadline = DateTime.UtcNow.AddSeconds 5.0
        while not server.IsRunning && DateTime.UtcNow < deadline do
            System.Threading.Thread.Sleep 2
        (server :> IDisposable).Dispose()
        Assert.False server.IsRunning

    [<Fact>]
    let ``stop stops a running server`` () =
        use server =
            Application.create()
            |> Application.get "/" (fun _ -> Response.text "ok")
            |> Application.bind "127.0.0.1" 0us
        server.ServeBackground()
        let deadline = DateTime.UtcNow.AddSeconds 5.0
        while not server.IsRunning && DateTime.UtcNow < deadline do
            System.Threading.Thread.Sleep 2
        server.Stop()
        System.Threading.Thread.Sleep 50
        Assert.False server.IsRunning

    [<Fact>]
    let ``multiple independent servers can coexist`` () =
        use s1 =
            Application.create()
            |> Application.get "/" (fun _ -> Response.text "s1")
            |> Application.bind "127.0.0.1" 0us
        use s2 =
            Application.create()
            |> Application.get "/" (fun _ -> Response.text "s2")
            |> Application.bind "127.0.0.1" 0us
        Assert.NotEqual(s1.LocalPort, s2.LocalPort)

// ═════════════════════════════════════════════════════════════════════════════
// GROUP 4 — End-to-end HTTP tests
// ═════════════════════════════════════════════════════════════════════════════
//
// A single server is shared across all E2E tests via ServerFixture (IClassFixture).
// This cuts startup cost and keeps the test run fast.
//
// E2E WATCHDOG
// ─────────────
// A 30-second System.Threading.Timer fires Environment.Exit(1) to prevent the
// test run from hanging in CI if a deadlock occurs. The timer is disarmed in
// ServerFixture.Dispose.

type ServerFixture() =

    // ── Build the test application ────────────────────────────────────────────

    let appName = "conduit-fsharp-test"
    let version = "0.1.0"

    // After-hook: stamp every response with x-served-by.
    let afterHook = fun (req: Request) (resp: Response) ->
        resp |> Response.withHeader "x-served-by" $"{appName}/{version}"

    // Before-filter: block /maintenance with 503.
    let beforeFilter = fun (req: Request) ->
        if req.Path = "/maintenance" then
            Some (Response.text "Down for maintenance" |> Response.withStatus 503)
        else None

    let server =
        Application.create()
        |> Application.set "app_name" appName
        |> Application.set "version"  version
        |> Application.after afterHook
        |> Application.before beforeFilter
        |> Application.get "/" (fun _ -> Response.html $"<h1>Hello from {appName}</h1>")
        |> Application.get "/api/:id" (fun req ->
            let id = req.Param "id" |> Option.defaultValue "unknown"
            Response.json (JsonSerializer.Serialize {| id = id |}))
        |> Application.post "/api/echo" (fun req ->
            let ct = req.ContentType
            let ct2 =
                if   ct.StartsWith "application/json" then "application/json"
                elif ct.StartsWith "text/plain"       then "text/plain; charset=utf-8"
                else "application/octet-stream"
            Response.respond 200 (req.BodyString()) [("content-type", ct2)])
        |> Application.get "/search" (fun req ->
            let q = req.Query "q" |> Option.defaultValue ""
            Response.json (JsonSerializer.Serialize {| query = q |}))
        |> Application.get "/redirect" (fun _ -> Response.redirect "/")
        |> Application.get "/halt-418" (fun _ ->
            raise (HaltException (Response.text "I'm a teapot" |> Response.withStatus 418)))
        |> Application.get "/error-trigger" (fun _ ->
            raise (InvalidOperationException "test error from handler"))
        |> Application.notFound (fun req ->
            Response.json
                (JsonSerializer.Serialize {| error = "not found"; path = req.Path |})
            |> Response.withStatus 404)
        |> Application.onError (fun req ->
            eprintfn $"[test] handler error: {req.Error}"
            Response.json "{\"error\":\"internal server error\"}" |> Response.withStatus 500)
        |> Application.bind "127.0.0.1" 0us

    do
        server.ServeBackground()

        // Wait for the conduit-capi reactor to be live before sending requests.
        // (conduit-capi runs `embeddable-http-server`'s single inline reactor on
        // one background OS thread — NOT a Tokio pool; `IsRunning` flips true as
        // soon as conduit_server_serve_background spawns that thread.)
        //
        // NB: the ~40% "cold run" flake this fixture used to hit was NOT a
        // conduit-capi thread-init race (an earlier comment here blamed a
        // non-existent "Tokio worker pool"). The real cause was xUnit running this
        // E2E collection *concurrently* with the server-binding ServerLifecycleTests
        // collection: multiple conduit-capi servers + reactor threads competing for
        // CPU in one process starved this fixture's early requests and desynced
        // responses. `xunit.runner.json` (parallelizeTestCollections: false) now
        // serialises the collections, which makes the suite deterministic.
        let rdy = DateTime.UtcNow.AddSeconds 5.0
        while not server.IsRunning && DateTime.UtcNow < rdy do
            System.Threading.Thread.Sleep 2

    let baseUrl = $"http://127.0.0.1:{server.LocalPort}"
    let http    = new HttpClient(BaseAddress = Uri baseUrl)

    // 30-second watchdog: kills the process if E2E tests hang in CI.
    let watchdog =
        new System.Threading.Timer(
            (fun _ ->
                eprintfn "[watchdog] E2E tests timed out — aborting"
                Environment.Exit 1),
            null,
            TimeSpan.FromSeconds 30.0,
            System.Threading.Timeout.InfiniteTimeSpan)

    member _.Http    = http
    member _.BaseUrl = baseUrl

    interface IDisposable with
        member _.Dispose() =
            watchdog.Dispose()
            http.Dispose()
            (server :> IDisposable).Dispose()


[<Collection("E2E")>]
type EndToEndTests(fx: ServerFixture) =

    // F# requires let/do bindings before interface declarations in class bodies.
    let get  (path: string)                          = fx.Http.GetAsync(path).Result
    let post (path: string) (body: string) (ct: string) =
        fx.Http.PostAsync(path, new StringContent(body, Encoding.UTF8, ct)).Result

    interface IClassFixture<ServerFixture>

    // ── Root route ────────────────────────────────────────────────────────────

    [<Fact>]
    member _.``Root_ReturnsHtml`` () =
        let resp = get "/"
        Assert.Equal(HttpStatusCode.OK, resp.StatusCode)
        let ct = resp.Content.Headers.ContentType.MediaType
        Assert.Equal("text/html", ct)

    // ── Route parameters ──────────────────────────────────────────────────────

    [<Fact>]
    member _.``Param_ReturnsCorrectId`` () =
        let resp = get "/api/42"
        Assert.Equal(HttpStatusCode.OK, resp.StatusCode)
        let body = resp.Content.ReadAsStringAsync().Result
        Assert.Contains("42", body)

    // ── POST / echo ───────────────────────────────────────────────────────────

    [<Fact>]
    member _.``Post_EchoReflectsBody`` () =
        let resp = post "/api/echo" "{\"k\":1}" "application/json"
        Assert.Equal(HttpStatusCode.OK, resp.StatusCode)
        let body = resp.Content.ReadAsStringAsync().Result
        Assert.Equal("{\"k\":1}", body)

    [<Fact>]
    member _.``EchoEndpoint_UnknownContentType_Normalised`` () =
        let resp = post "/api/echo" "raw bytes" "application/octet-stream"
        Assert.Equal(HttpStatusCode.OK, resp.StatusCode)
        let ct = resp.Content.Headers.ContentType.MediaType
        Assert.Equal("application/octet-stream", ct)

    // ── Query string ──────────────────────────────────────────────────────────

    [<Fact>]
    member _.``Query_ReturnsQueryValue`` () =
        let resp = get "/search?q=hello"
        Assert.Equal(HttpStatusCode.OK, resp.StatusCode)
        let body = resp.Content.ReadAsStringAsync().Result
        Assert.Contains("hello", body)

    [<Fact>]
    member _.``Query_MissingParam_DefaultsToEmpty`` () =
        let resp = get "/search"
        Assert.Equal(HttpStatusCode.OK, resp.StatusCode)
        let body = resp.Content.ReadAsStringAsync().Result
        Assert.Contains("\"query\":\"\"", body)

    // ── Redirect ──────────────────────────────────────────────────────────────

    [<Fact>]
    member _.``Redirect_Returns302`` () =
        use noFollow = new HttpClient(
            new HttpClientHandler(AllowAutoRedirect = false),
            BaseAddress = Uri fx.BaseUrl)
        let resp = noFollow.GetAsync("/redirect").Result
        Assert.Equal(HttpStatusCode.Redirect, resp.StatusCode)
        Assert.NotNull(resp.Headers.Location)

    // ── Before-filter ─────────────────────────────────────────────────────────

    [<Fact>]
    member _.``BeforeFilter_MaintenanceRoute_Returns503`` () =
        let resp = get "/maintenance"
        Assert.Equal(HttpStatusCode.ServiceUnavailable, resp.StatusCode)

    [<Fact>]
    member _.``BeforeFilter_NormalRoute_PassesThrough`` () =
        let resp = get "/"
        Assert.Equal(HttpStatusCode.OK, resp.StatusCode)

    // ── After-hook ────────────────────────────────────────────────────────────

    [<Fact>]
    member _.``AfterHook_StampsXServedBy`` () =
        let resp = get "/"
        let header = resp.Headers.TryGetValues("x-served-by") |> snd |> Seq.tryHead
        Assert.True(header.IsSome)
        Assert.Contains("conduit-fsharp-test", header.Value)

    // ── Not-found ─────────────────────────────────────────────────────────────

    [<Fact>]
    member _.``NotFound_ReturnsCustom404`` () =
        let resp = get "/no-such-route"
        Assert.Equal(HttpStatusCode.NotFound, resp.StatusCode)
        let body = resp.Content.ReadAsStringAsync().Result
        Assert.Contains("not found", body)
        Assert.Contains("/no-such-route", body)

    // ── HaltException ─────────────────────────────────────────────────────────

    [<Fact>]
    member _.``Halt_Returns418`` () =
        let resp = get "/halt-418"
        Assert.Equal(enum<HttpStatusCode> 418, resp.StatusCode)

    // ── Error handler ─────────────────────────────────────────────────────────

    [<Fact>]
    member _.``ErrorHandler_SuppressesRawError`` () =
        let resp = get "/error-trigger"
        Assert.Equal(HttpStatusCode.InternalServerError, resp.StatusCode)
        let body = resp.Content.ReadAsStringAsync().Result
        // The handler threw InvalidOperationException("test error from handler").
        // That detail must NOT appear in the response — only a generic 500.
        Assert.DoesNotContain("test error from handler", body)
        Assert.Contains("internal server error", body)

    // ── Content-type reflection ───────────────────────────────────────────────

    [<Fact>]
    member _.``Echo_JsonContentType_Preserved`` () =
        let resp = post "/api/echo" "{}" "application/json"
        let ct = resp.Content.Headers.ContentType.MediaType
        Assert.Equal("application/json", ct)

    [<Fact>]
    member _.``Echo_TextContentType_Preserved`` () =
        let resp = post "/api/echo" "hello" "text/plain"
        let ct = resp.Content.Headers.ContentType.MediaType
        Assert.Equal("text/plain", ct)
