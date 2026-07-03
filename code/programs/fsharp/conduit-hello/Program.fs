// conduit-hello — demonstration of CodingAdventures.Conduit.FSharp (WEB16)
//
// This program shows the idiomatic F# usage pattern:
//   1. Create an Application builder.
//   2. Read settings BEFORE bind() — the ConduitApp* is consumed on bind().
//   3. Register routes using lambdas that capture read settings as local values.
//   4. Register before-filters and after-hooks using |> pipe operators.
//   5. Bind to a port and call Serve() (blocks until Ctrl-C).
//
// Run:  CONDUIT_CAPI_PATH=<path-to-lib> dotnet run
// Test: sh tools/run-tests.sh

module ConduitHello.Program

open System
open System.Net
open System.Text.Json
open CodingAdventures.Conduit.FSharp

// ── Application setup ────────────────────────────────────────────────────────

let app = Application.create()

// Store runtime configuration as named settings.
let app2 =
    app
    |> Application.set "app_name" "conduit-hello"
    |> Application.set "version"  "0.1.0"
    |> Application.set "env"      (Environment.GetEnvironmentVariable "APP_ENV" |> Option.ofObj |> Option.defaultValue "development")

// IMPORTANT: read settings now — after bind(), the ConduitApp* is gone.
let appName = Application.getSetting "app_name" app2 |> Option.defaultValue "conduit-hello"
let version = Application.getSetting "version"  app2 |> Option.defaultValue "0.1.0"
let env     = Application.getSetting "env"      app2 |> Option.defaultValue "development"

// HTML-encode all server-controlled values embedded in the HTML template.
// Defence-in-depth: even operator-supplied env/version values must not
// be trusted to be free of HTML metacharacters.
let safeAppName = WebUtility.HtmlEncode appName
let safeVersion = WebUtility.HtmlEncode version
let safeEnv     = WebUtility.HtmlEncode env

// ── Before-filter: simple API-key guard ──────────────────────────────────────
//
// Return None to pass through; return Some response to short-circuit.

let apiKeyFilter (req: Request) =
    // Let static assets and the health check pass without auth.
    if req.Path = "/health" || req.Path = "/" then None
    else
        // Opt-in to bypass, not opt-in to enforcement. Enforce auth in all
        // environments except the explicitly-whitelisted "development" value.
        // This way a misconfigured deploy that forgets to set APP_ENV=production
        // is still protected.
        let key = req.Header "x-api-key"
        if env <> "development" && key.IsNone then
            Some (Response.json "{\"error\":\"missing x-api-key header\"}" |> Response.withStatus 401)
        else None

// ── After-hook: stamp every response with server metadata ─────────────────────

let metaHook (req: Request) (resp: Response) =
    resp
    |> Response.withHeader "x-served-by"            $"{appName}/{version}"
    |> Response.withHeader "x-env"                   env
    |> Response.withHeader "x-content-type-options" "nosniff"

// ── Routes ────────────────────────────────────────────────────────────────────

let server =
    app2
    |> Application.before apiKeyFilter
    |> Application.after  metaHook
    // Home page — demonstrates HTML response.
    |> Application.get "/" (fun _ ->
        Response.html $"""
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
            """)
    // Health check — used by load balancers / orchestration.
    |> Application.get "/health" (fun _ ->
        Response.json (JsonSerializer.Serialize {|
            status  = "ok"
            name    = appName
            version = version
            env     = env
        |}))
    // Route parameter — /api/greet/:name
    |> Application.get "/api/greet/:name" (fun req ->
        let name = req.Param "name" |> Option.defaultValue "stranger"
        Response.json (JsonSerializer.Serialize {|
            greeting = $"Hello, {name}!"
            from     = appName
        |}))
    // Query string — /api/search?q=…&limit=…
    |> Application.get "/api/search" (fun req ->
        let q     = req.Query "q"     |> Option.defaultValue ""
        let limitStr = req.Query "limit" |> Option.defaultValue "10"
        let n =
            match Int32.TryParse limitStr with
            | true, v when v >= 1 && v <= 100 -> v
            | _ -> 10
        Response.json (JsonSerializer.Serialize {|
            query   = q
            limit   = n
            results = Array.empty<string>
        |}))
    // Echo body — demonstrates POST request body access.
    // Only mirrors safe content types to avoid content-sniffing attacks.
    |> Application.post "/api/echo" (fun req ->
        let ct = req.ContentType
        let ct2 =
            if   ct.StartsWith "application/json" then "application/json"
            elif ct.StartsWith "text/plain"        then "text/plain; charset=utf-8"
            else "application/octet-stream"
        Response.respond 200 (req.BodyString()) [("content-type", ct2)])
    // Redirect — demonstrates 3xx responses.
    |> Application.get "/old-home" (fun _ -> Response.redirect "/")
    // Teapot — demonstrates non-local exit via HaltException.
    |> Application.get "/tpot" (fun _ ->
        raise (HaltException (Response.text "I'm a teapot" |> Response.withStatus 418)))
    // ── Error handling ────────────────────────────────────────────────────────
    |> Application.notFound (fun req ->
        Response.json (JsonSerializer.Serialize {|
            error = "not found"
            path  = req.Path
        |}) |> Response.withStatus 404)
    |> Application.onError (fun req ->
        // Log the real error server-side; never expose it to clients.
        eprintfn $"[{appName}] handler error: {req.Error}"
        Response.json "{\"error\":\"internal server error\"}" |> Response.withStatus 500)
    // ── Bind and serve ────────────────────────────────────────────────────────
    |> Application.bind
        (Environment.GetEnvironmentVariable "HOST" |> Option.ofObj |> Option.defaultValue "127.0.0.1")
        (match UInt16.TryParse (Environment.GetEnvironmentVariable "PORT" |> Option.ofObj |> Option.defaultValue "3000") with
         | true, p when p > 0us -> p
         | _ ->
            eprintfn $"[{appName}] invalid PORT; defaulting to 3000"
            3000us)

printfn $"[{appName}] listening on port {server.LocalPort} (env={env})"
server.Serve()
