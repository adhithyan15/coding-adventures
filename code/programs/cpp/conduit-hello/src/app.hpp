// ============================================================================
// app.hpp — the demo application, exercising the full Conduit C++ DSL.
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
// Kept in a header so the smoke test can build and drive the same app.

#ifndef CONDUIT_HELLO_APP_HPP
#define CONDUIT_HELLO_APP_HPP

#include <stdexcept>

#include "conduit/conduit.hpp"

inline conduit::Application make_app() {
    using namespace conduit;
    Application app;
    app.set("app_name", "Conduit Hello");

    app.before([](const Request& req) -> std::optional<Response> {
        if (req.path() == "/down") halt(503, "Under maintenance");
        return std::nullopt;
    });

    // After hook: stamp a header on every response.
    app.after([](const Request&, Response resp) {
        resp.headers.emplace_back("x-served-by", "conduit-hello");
        return resp;
    });

    app.get("/", [](const Request&) {
        return Response::html("<h1>Hello from Conduit (C++)!</h1><p>Try <code>/hello/Ada</code></p>");
    });

    app.get("/hello/:name", [](const Request& req) {
        return Response::json("{\"message\":\"Hello " + req.param("name").value_or("") +
                              "\",\"app\":\"Conduit\"}");
    });

    app.post("/echo", [](const Request& req) {
        std::string ct = req.contentType().empty() ? "text/plain" : req.contentType();
        return Response::respond(200, req.body(), {{"content-type", ct}});
    });

    app.get("/search", [](const Request& req) {
        return Response::text("you searched for: " + req.query("q").value_or("(nothing)"));
    });

    app.get("/redirect", [](const Request&) { return Response::redirect("/", 301); });

    app.get("/halt", [](const Request&) -> Response {
        halt(403, "Forbidden — this route always halts");
    });

    app.get("/down", [](const Request&) {
        return Response::text("unreachable — the before filter halts first");
    });

    app.get("/error", [](const Request&) -> Response { throw std::runtime_error("boom"); });

    app.notFound([](const Request& req) { return Response::text("No such route: " + req.path(), 404); });
    app.onError([](const Request&) { return Response::json("{\"error\":\"internal server error\"}", 500); });

    return app;
}

#endif  // CONDUIT_HELLO_APP_HPP
