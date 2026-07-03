package com.codingadventures.conduit;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;
import java.util.concurrent.TimeUnit;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;
import org.junit.jupiter.api.Timeout;

/**
 * End-to-end tests: a real server on an OS-assigned port, driven over HTTP with
 * {@link HttpClient}. One application with every feature is started once and
 * shared across the test methods.
 *
 * <p>A class-level {@link Timeout} fails any test (or lifecycle method) that
 * runs longer than 30s, so a hung server/dispatch surfaces as a clear failure
 * instead of stalling the whole CI job toward the multi-hour default limit.
 */
@TestInstance(TestInstance.Lifecycle.PER_CLASS)
@Timeout(value = 30, unit = TimeUnit.SECONDS)
class ServerTest {

    private Application app;
    private Server server;
    private int port;
    private final HttpClient client = HttpClient.newBuilder()
            .followRedirects(HttpClient.Redirect.NEVER)
            .connectTimeout(Duration.ofSeconds(5))
            .build();

    @BeforeAll
    void startServer() throws Exception {
        app = new Application();
        app.before(req -> req.path().equals("/down") ? Responses.halt(503, "Maintenance") : null)
                .get("/", req -> Responses.html("<h1>OK</h1>"))
                .get("/ping", req -> Responses.text("pong"))
                .get("/health", req -> Responses.json("{\"ok\":true}"))
                .get("/hello/:name", req -> Responses.text("Hello " + req.param("name")))
                .get("/search", req -> Responses.text("q=" + req.queryParam("q")))
                .post("/echo", req -> Responses.respond(200, req.body(),
                        java.util.Map.of("content-type", "text/plain; charset=utf-8")))
                .get("/old", req -> Responses.redirect("/new"))
                .get("/forbidden", req -> Responses.halt(403, "Forbidden"))
                .get("/deephalt", req -> {
                    throw new HaltException(401, "deep");
                })
                .get("/boom", req -> {
                    throw new RuntimeException("kaboom");
                })
                .notFound(req -> Responses.html("Not Found: " + req.path(), 404))
                .onError(req -> Responses.json("{\"error\":\"" + req.error() + "\"}", 500));

        server = Server.bind(app, "127.0.0.1", 0);
        server.serveBackground();
        // Give the background server a moment to bind/accept.
        Thread.sleep(150);
        port = server.localPort();
    }

    @AfterAll
    void stopServer() {
        if (server != null) {
            server.stop();
            server.close();
        }
        if (app != null) {
            app.close();
        }
    }

    private HttpResponse<String> get(String path) throws Exception {
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create("http://127.0.0.1:" + port + path))
                .timeout(Duration.ofSeconds(5))
                .GET()
                .build();
        return client.send(request, HttpResponse.BodyHandlers.ofString());
    }

    private HttpResponse<String> post(String path, String body) throws Exception {
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create("http://127.0.0.1:" + port + path))
                .timeout(Duration.ofSeconds(5))
                .header("content-type", "text/plain")
                .POST(HttpRequest.BodyPublishers.ofString(body))
                .build();
        return client.send(request, HttpResponse.BodyHandlers.ofString());
    }

    @Test
    void rootReturnsHtml() throws Exception {
        HttpResponse<String> r = get("/");
        assertEquals(200, r.statusCode());
        assertEquals("<h1>OK</h1>", r.body());
        assertTrue(r.headers().firstValue("content-type").orElse("").contains("text/html"));
    }

    @Test
    void pingReturnsText() throws Exception {
        assertEquals("pong", get("/ping").body());
    }

    @Test
    void healthReturnsJson() throws Exception {
        HttpResponse<String> r = get("/health");
        assertEquals(200, r.statusCode());
        assertTrue(r.body().contains("\"ok\":true"));
    }

    @Test
    void routeParamCaptured() throws Exception {
        assertEquals("Hello Adhithya", get("/hello/Adhithya").body());
    }

    @Test
    void queryParamsParsed() throws Exception {
        assertEquals("q=hello", get("/search?q=hello&n=5").body());
    }

    @Test
    void echoReturnsBody() throws Exception {
        HttpResponse<String> r = post("/echo", "hello world");
        assertEquals(200, r.statusCode());
        assertEquals("hello world", r.body());
    }

    @Test
    void beforeFilterHalts() throws Exception {
        HttpResponse<String> r = get("/down");
        assertEquals(503, r.statusCode());
        assertEquals("Maintenance", r.body());
    }

    @Test
    void redirectReturns302WithLocation() throws Exception {
        HttpResponse<String> r = get("/old");
        assertEquals(302, r.statusCode());
        assertEquals("/new", r.headers().firstValue("location").orElse(""));
    }

    @Test
    void haltResponseReturns403() throws Exception {
        HttpResponse<String> r = get("/forbidden");
        assertEquals(403, r.statusCode());
        assertEquals("Forbidden", r.body());
    }

    @Test
    void thrownHaltExceptionReturnsStatus() throws Exception {
        HttpResponse<String> r = get("/deephalt");
        assertEquals(401, r.statusCode());
        assertEquals("deep", r.body());
    }

    @Test
    void thrownErrorRoutesToErrorHandler() throws Exception {
        HttpResponse<String> r = get("/boom");
        assertEquals(500, r.statusCode());
        assertTrue(r.body().contains("kaboom"), r.body());
    }

    @Test
    void notFoundHandlerRuns() throws Exception {
        HttpResponse<String> r = get("/missing");
        assertEquals(404, r.statusCode());
        assertTrue(r.body().contains("Not Found: /missing"));
    }

    @Test
    void serverMetadata() {
        assertTrue(port > 0);
        assertTrue(server.running());
    }

    @Test
    void runningTogglesAfterStop() throws Exception {
        try (Application a = new Application()) {
            a.get("/", req -> Responses.text("ok"));
            Server s = Server.bind(a, "127.0.0.1", 0);
            s.serveBackground();
            // Let the background accept loop fully enter its event wait before
            // stopping, so stop() reliably wakes it (mirrors the @BeforeAll
            // pattern). Stopping the instant after serveBackground() can race
            // the loop's startup on some platforms.
            Thread.sleep(150);
            assertTrue(s.running());
            s.stop();
            assertFalse(s.running());
            s.close();
        }
    }
}
