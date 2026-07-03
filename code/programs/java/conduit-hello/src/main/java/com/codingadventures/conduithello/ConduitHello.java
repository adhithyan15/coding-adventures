package com.codingadventures.conduithello;

import static com.codingadventures.conduit.Responses.halt;
import static com.codingadventures.conduit.Responses.html;
import static com.codingadventures.conduit.Responses.json;
import static com.codingadventures.conduit.Responses.redirect;
import static com.codingadventures.conduit.Responses.respond;

import com.codingadventures.conduit.Application;
import com.codingadventures.conduit.Server;
import java.util.Map;

/**
 * Conduit demo — eight routes exercising every framework feature, mirroring the
 * Ruby/Python/Lua/TypeScript/Elixir/Rust {@code conduit-hello} demos.
 *
 * <pre>
 *   GET  /              HTML home
 *   GET  /hello/:name   JSON greeting using a route param
 *   POST /echo          echoes the request body
 *   GET  /redirect      301 to /
 *   GET  /halt          halt(403, "Forbidden")
 *   GET  /down          before-filter halt(503)
 *   GET  /error         throws → routes to the error handler (500)
 *   (any)/missing       custom not-found handler (404)
 * </pre>
 *
 * Run with {@code gradle run} (the build sets {@code java.library.path}).
 */
public final class ConduitHello {

    private ConduitHello() {
    }

    /** Build the demo application. Pure — easy to unit-test. */
    public static Application app() {
        Application app = new Application();
        app.before(req -> req.path().equals("/down") ? halt(503, "Under maintenance") : null)
                .get("/", req -> html("""
                        <!DOCTYPE html>
                        <html><head><title>Conduit Hello</title></head>
                        <body>
                          <h1>Hello from Conduit (Java)!</h1>
                          <p>Try <a href="/hello/Adhithya">/hello/Adhithya</a>.</p>
                        </body></html>
                        """))
                .get("/hello/:name", req ->
                        json("{\"message\":\"Hello " + req.param("name") + "\"}"))
                .post("/echo", req ->
                        respond(200, req.body(), Map.of("content-type", req.contentType())))
                .get("/redirect", req -> redirect("/", 301))
                .get("/halt", req -> halt(403, "Forbidden"))
                .get("/error", req -> {
                    throw new RuntimeException("Something went wrong!");
                })
                .notFound(req -> html("<h1>Not Found: " + req.path() + "</h1>", 404))
                .onError(req ->
                        json("{\"error\":\"Internal Server Error\",\"detail\":\"" + req.error() + "\"}", 500))
                .set("app_name", "Conduit Hello (Java)");
        return app;
    }

    /** Run the demo server in the foreground (blocks). */
    public static void main(String[] args) {
        String host = "127.0.0.1";
        int port = 3000;
        for (int i = 0; i < args.length - 1; i++) {
            if (args[i].equals("--host")) {
                host = args[i + 1];
            } else if (args[i].equals("--port")) {
                port = Integer.parseInt(args[i + 1]);
            }
        }
        Application app = app();
        Server server = Server.bind(app, host, port);
        System.out.println("Conduit Hello listening on http://" + host + ":" + port);
        server.serve();
    }
}
