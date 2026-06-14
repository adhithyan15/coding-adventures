import Conduit

// ============================================================================
// makeApp — the demo application, exercising the full Conduit DSL.
// ============================================================================
//
//   GET  /                  → HTML greeting
//   GET  /hello/:name       → JSON with the route param
//   POST /echo              → echoes the body, content-type passthrough
//   GET  /search?q=...      → reads a query param
//   GET  /redirect          → 301 to /
//   GET  /halt              → 403 via halt()
//   GET  /down              → 503 via a before filter (short-circuits)
//   GET  /error             → throws → routed to the custom error handler (500)
//   GET  /<anything-else>   → custom 404 handler
//
// Kept separate from main.swift so the smoke test can build and drive it.

enum DemoError: Error { case boom }

func makeApp() -> Application {
    let app = Application()
    app.set("app_name", "Conduit Hello")

    // Before filter: short-circuit /down with a 503.
    app.before { req in
        if req.path == "/down" { try halt(503, "Under maintenance") }
        return nil
    }

    // After hook: stamp a header on every response (read-only-style use).
    app.after { _, resp in
        var r = resp
        r.headers.append(("x-served-by", "conduit-hello"))
        return r
    }

    app.get("/") { _ in
        .html("<h1>Hello from Conduit (Swift)!</h1><p>Try <code>/hello/Ada</code></p>")
    }

    app.get("/hello/:name") { req in
        .json("{\"message\":\"Hello \(req.param("name") ?? "")\",\"app\":\"Conduit\"}")
    }

    app.post("/echo") { req in
        .respond(200, req.bodyText,
                 headers: [("content-type", req.contentType.isEmpty ? "text/plain" : req.contentType)])
    }

    app.get("/search") { req in
        .text("you searched for: \(req.query("q") ?? "(nothing)")")
    }

    app.get("/redirect") { _ in try .redirect("/", status: 301) }

    app.get("/halt") { _ in try halt(403, "Forbidden — this route always halts") }

    app.get("/down") { _ in .text("unreachable — the before filter halts first") }

    app.get("/error") { _ in throw DemoError.boom }

    app.notFound { req in .text("No such route: \(req.path)", status: 404) }
    app.onError { _ in .json("{\"error\":\"internal server error\"}", status: 500) }

    return app
}
