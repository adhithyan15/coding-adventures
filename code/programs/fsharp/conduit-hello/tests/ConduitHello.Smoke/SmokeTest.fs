// SmokeTest.fs — integration smoke tests for conduit-hello (F#)
//
// Spins up a conduit-hello–style server inline (rather than shelling out to
// `dotnet run`) to verify the demo application logic end-to-end.

module ConduitHello.SmokeTest

open System
open System.Net
open System.Net.Http
open System.Text
open CodingAdventures.Conduit.FSharp
open Xunit

// ── Shared server fixture ─────────────────────────────────────────────────────

type SmokeFixture() =

    let appName = "conduit-hello"
    let version = "0.1.0"
    let env     = "development"

    let safeAppName = System.Net.WebUtility.HtmlEncode appName
    let safeVersion = System.Net.WebUtility.HtmlEncode version
    let safeEnv     = System.Net.WebUtility.HtmlEncode env

    let server =
        Application.create()
        |> Application.set "app_name" appName
        |> Application.set "version"  version
        |> Application.set "env"      env
        |> Application.before (fun req ->
            if req.Path = "/health" || req.Path = "/" then None
            else
                let key = req.Header "x-api-key"
                if env <> "development" && key.IsNone then
                    Some (Response.json "{\"error\":\"missing x-api-key header\"}" |> Response.withStatus 401)
                else None)
        |> Application.after (fun _ resp ->
            resp
            |> Response.withHeader "x-served-by" $"{appName}/{version}"
            |> Response.withHeader "x-env" env)
        |> Application.get "/" (fun _ ->
            Response.html $"""
                <!doctype html>
                <html><body>
                  <h1>{safeAppName}</h1>
                  <p>Version: {safeVersion} | Env: {safeEnv}</p>
                </body></html>
                """)
        |> Application.get "/health" (fun _ ->
            Response.json $"{{\"status\":\"ok\",\"name\":\"{appName}\",\"version\":\"{version}\",\"env\":\"{env}\"}}")
        |> Application.get "/api/greet/:name" (fun req ->
            let name = req.Param "name" |> Option.defaultValue "stranger"
            Response.json $"{{\"greeting\":\"Hello, {name}!\",\"from\":\"{appName}\"}}")
        |> Application.get "/api/search" (fun req ->
            let q = req.Query "q" |> Option.defaultValue ""
            Response.json (sprintf "{\"query\":\"%s\",\"limit\":10,\"results\":[]}" q))
        |> Application.post "/api/echo" (fun req ->
            let ct = req.ContentType
            let ct2 =
                if   ct.StartsWith "application/json" then "application/json"
                elif ct.StartsWith "text/plain"        then "text/plain; charset=utf-8"
                else "application/octet-stream"
            Response.respond 200 (req.BodyString()) [("content-type", ct2)])
        |> Application.get "/old-home" (fun _ -> Response.redirect "/")
        |> Application.get "/tpot" (fun _ -> raise (HaltException (Response.text "I'm a teapot" |> Response.withStatus 418)))
        |> Application.notFound (fun req ->
            Response.json $"{{\"error\":\"not found\",\"path\":\"{req.Path}\"}}" |> Response.withStatus 404)
        |> Application.onError (fun req ->
            eprintfn $"[smoke] error: {req.Error}"
            Response.json "{\"error\":\"internal server error\"}" |> Response.withStatus 500)
        |> Application.bind "127.0.0.1" 0us

    do
        server.ServeBackground()
        let deadline = DateTime.UtcNow.AddSeconds 5.0
        while not server.IsRunning && DateTime.UtcNow < deadline do
            System.Threading.Thread.Sleep 2

    let baseUrl = $"http://127.0.0.1:{server.LocalPort}"
    let http    = new HttpClient(BaseAddress = Uri baseUrl)

    let watchdog =
        new System.Threading.Timer(
            (fun _ -> eprintfn "[smoke watchdog] timed out"; Environment.Exit 1),
            null,
            TimeSpan.FromSeconds 30.0,
            System.Threading.Timeout.InfiniteTimeSpan)

    member _.Http = http
    member _.BaseUrl = baseUrl

    interface IDisposable with
        member _.Dispose() =
            watchdog.Dispose()
            http.Dispose()
            (server :> IDisposable).Dispose()


[<Collection("Smoke")>]
type SmokeTests(fx: SmokeFixture) =

    // F# requires let/do bindings before interface declarations in class bodies.
    let get  (path: string)                            = fx.Http.GetAsync(path).Result
    let post (path: string) (body: string) (ct: string) =
        fx.Http.PostAsync(path, new StringContent(body, Encoding.UTF8, ct)).Result

    interface IClassFixture<SmokeFixture>

    [<Fact>]
    member _.``home page returns HTML`` () =
        let resp = get "/"
        Assert.Equal(HttpStatusCode.OK, resp.StatusCode)
        Assert.Equal("text/html", resp.Content.Headers.ContentType.MediaType)
        let body = resp.Content.ReadAsStringAsync().Result
        Assert.Contains("conduit-hello", body)

    [<Fact>]
    member _.``health check returns ok`` () =
        let resp = get "/health"
        Assert.Equal(HttpStatusCode.OK, resp.StatusCode)
        let body = resp.Content.ReadAsStringAsync().Result
        Assert.Contains("\"status\":\"ok\"", body)

    [<Fact>]
    member _.``greet route returns personalised greeting`` () =
        let resp = get "/api/greet/Alice"
        Assert.Equal(HttpStatusCode.OK, resp.StatusCode)
        let body = resp.Content.ReadAsStringAsync().Result
        Assert.Contains("Alice", body)

    [<Fact>]
    member _.``search route returns query`` () =
        let resp = get "/api/search?q=fsharp"
        Assert.Equal(HttpStatusCode.OK, resp.StatusCode)
        let body = resp.Content.ReadAsStringAsync().Result
        Assert.Contains("fsharp", body)

    [<Fact>]
    member _.``echo route mirrors body`` () =
        let resp = post "/api/echo" "{\"x\":1}" "application/json"
        Assert.Equal(HttpStatusCode.OK, resp.StatusCode)
        let body = resp.Content.ReadAsStringAsync().Result
        Assert.Equal("{\"x\":1}", body)

    [<Fact>]
    member _.``old-home redirects`` () =
        use noFollow = new HttpClient(
            new HttpClientHandler(AllowAutoRedirect = false),
            BaseAddress = Uri fx.BaseUrl)
        let resp = noFollow.GetAsync("/old-home").Result
        Assert.Equal(HttpStatusCode.Redirect, resp.StatusCode)

    [<Fact>]
    member _.``teapot returns 418`` () =
        let resp = get "/tpot"
        Assert.Equal(enum<HttpStatusCode> 418, resp.StatusCode)

    [<Fact>]
    member _.``unknown route returns 404`` () =
        let resp = get "/no-such-route"
        Assert.Equal(HttpStatusCode.NotFound, resp.StatusCode)
        let body = resp.Content.ReadAsStringAsync().Result
        Assert.Contains("not found", body)

    [<Fact>]
    member _.``x-served-by header present on all responses`` () =
        let resp = get "/health"
        let ok, vals = resp.Headers.TryGetValues "x-served-by"
        Assert.True ok
        Assert.Contains("conduit-hello", Seq.head vals)
