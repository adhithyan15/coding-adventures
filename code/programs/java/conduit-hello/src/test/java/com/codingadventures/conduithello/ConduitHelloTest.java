package com.codingadventures.conduithello;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.codingadventures.conduit.Application;
import com.codingadventures.conduit.Server;
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
 * Integration tests for the conduit-hello demo, driven over real HTTP.
 *
 * <p>A 30s class-level {@link Timeout} surfaces any hung server/dispatch as a
 * test failure instead of stalling CI toward the multi-hour job limit.
 */
@TestInstance(TestInstance.Lifecycle.PER_CLASS)
@Timeout(value = 30, unit = TimeUnit.SECONDS)
class ConduitHelloTest {

    private Application app;
    private Server server;
    private int port;
    private final HttpClient client = HttpClient.newBuilder()
            .followRedirects(HttpClient.Redirect.NEVER)
            .connectTimeout(Duration.ofSeconds(5))
            .build();

    @BeforeAll
    void start() throws Exception {
        app = ConduitHello.app();
        server = Server.bind(app, "127.0.0.1", 0);
        server.serveBackground();
        Thread.sleep(150);
        port = server.localPort();
    }

    @AfterAll
    void stop() {
        if (server != null) {
            server.stop();
            server.close();
        }
        if (app != null) {
            app.close();
        }
    }

    private HttpResponse<String> get(String path) throws Exception {
        return client.send(
                HttpRequest.newBuilder(URI.create("http://127.0.0.1:" + port + path))
                        .timeout(Duration.ofSeconds(5)).GET().build(),
                HttpResponse.BodyHandlers.ofString());
    }

    private HttpResponse<String> post(String path, String body) throws Exception {
        return client.send(
                HttpRequest.newBuilder(URI.create("http://127.0.0.1:" + port + path))
                        .timeout(Duration.ofSeconds(5))
                        .header("content-type", "text/plain")
                        .POST(HttpRequest.BodyPublishers.ofString(body)).build(),
                HttpResponse.BodyHandlers.ofString());
    }

    @Test
    void rootIsHtml() throws Exception {
        HttpResponse<String> r = get("/");
        assertEquals(200, r.statusCode());
        assertTrue(r.body().contains("Hello from Conduit"));
        assertTrue(r.body().contains("/hello/Adhithya"));
    }

    @Test
    void helloCapturesParam() throws Exception {
        assertTrue(get("/hello/Adhithya").body().contains("Hello Adhithya"));
    }

    @Test
    void helloWorks() throws Exception {
        assertTrue(get("/hello/World").body().contains("Hello World"));
    }

    @Test
    void echoReturnsBody() throws Exception {
        HttpResponse<String> r = post("/echo", "hello world");
        assertEquals(200, r.statusCode());
        assertEquals("hello world", r.body());
    }

    @Test
    void echoPreservesJsonBody() throws Exception {
        String payload = "{\"ping\":\"pong\"}";
        assertEquals(payload, post("/echo", payload).body());
    }

    @Test
    void redirectIs301ToRoot() throws Exception {
        HttpResponse<String> r = get("/redirect");
        assertEquals(301, r.statusCode());
        assertEquals("/", r.headers().firstValue("location").orElse(""));
    }

    @Test
    void haltIs403() throws Exception {
        HttpResponse<String> r = get("/halt");
        assertEquals(403, r.statusCode());
        assertEquals("Forbidden", r.body());
    }

    @Test
    void downTriggersBeforeFilter() throws Exception {
        HttpResponse<String> r = get("/down");
        assertEquals(503, r.statusCode());
        assertEquals("Under maintenance", r.body());
    }

    @Test
    void errorRoutesToErrorHandler() throws Exception {
        HttpResponse<String> r = get("/error");
        assertEquals(500, r.statusCode());
        assertTrue(r.body().contains("Internal Server Error"));
        assertTrue(r.body().contains("Something went wrong"), r.body());
    }

    @Test
    void missingReturnsCustomNotFound() throws Exception {
        HttpResponse<String> r = get("/missing");
        assertEquals(404, r.statusCode());
        assertTrue(r.body().contains("Not Found: /missing"));
    }

    @Test
    void anyUnknownPathIs404() throws Exception {
        assertEquals(404, get("/anything/else").statusCode());
    }

    @Test
    void appFactoryIsPureAndInspectable() {
        try (Application a = ConduitHello.app()) {
            assertEquals("Conduit Hello (Java)", a.getSetting("app_name"));
        }
    }

    @Test
    void localPortIsAssigned() {
        assertTrue(port > 0);
    }

    @Test
    void emptyEchoBody() throws Exception {
        assertEquals("", post("/echo", "").body());
    }
}
