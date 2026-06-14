package com.codingadventures.conduitkt

import java.net.URI
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import java.time.Duration
import java.util.concurrent.TimeUnit
import kotlin.test.AfterTest
import kotlin.test.Test
import kotlin.test.assertContains
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import org.junit.jupiter.api.Timeout

/**
 * Tests for the Kotlin Conduit DSL. The response-helper and builder paths are
 * exercised directly; the request handling is verified end-to-end over real
 * HTTP (the Java `Request` constructor is package-private, so the lazy-map
 * parsing is covered through the running server rather than in isolation).
 *
 * A 30s class-level timeout surfaces any hung server/dispatch as a clear
 * failure instead of stalling CI (same hardening as the Java port).
 */
@Timeout(value = 30, unit = TimeUnit.SECONDS)
class ConduitKtTest {

    private var appHandle: ConduitApp? = null

    @AfterTest
    fun tearDown() {
        appHandle?.let {
            it.stop()
            it.close()
        }
        appHandle = null
    }

    private val client: HttpClient = HttpClient.newBuilder()
        .followRedirects(HttpClient.Redirect.NEVER)
        .connectTimeout(Duration.ofSeconds(5))
        .build()

    private fun start(block: ConduitBuilder.() -> Unit): Int {
        val app = conduit(block)
        appHandle = app
        app.serveBackground("127.0.0.1", 0)
        Thread.sleep(150)
        return app.localPort()
    }

    private fun get(port: Int, path: String): HttpResponse<String> =
        client.send(
            HttpRequest.newBuilder(URI.create("http://127.0.0.1:$port$path"))
                .timeout(Duration.ofSeconds(5)).GET().build(),
            HttpResponse.BodyHandlers.ofString(),
        )

    private fun post(port: Int, path: String, body: String): HttpResponse<String> =
        client.send(
            HttpRequest.newBuilder(URI.create("http://127.0.0.1:$port$path"))
                .timeout(Duration.ofSeconds(5))
                .header("content-type", "text/plain")
                .POST(HttpRequest.BodyPublishers.ofString(body)).build(),
            HttpResponse.BodyHandlers.ofString(),
        )

    // ── Response-helper unit tests (no server) ──────────────────────────────

    @Test
    fun htmlHelper() {
        val r = html("<h1>Hi</h1>")
        assertEquals(200, r.status())
        assertEquals("<h1>Hi</h1>", r.body())
        assertEquals("text/html; charset=utf-8", r.headers()["content-type"])
    }

    @Test
    fun jsonAndTextAndRespondHelpers() {
        assertEquals("application/json", json("{}").headers()["content-type"])
        assertEquals(201, text("x", 201).status())
        val r = respond(204, "", mapOf("x-y" to "z"))
        assertEquals("z", r.headers()["x-y"])
    }

    @Test
    fun haltAndRedirectHelpers() {
        assertEquals(403, halt(403, "no").status())
        val r = redirect("/new", 301)
        assertEquals(301, r.status())
        assertEquals("/new", r.headers()["location"])
    }

    // ── E2E tests ────────────────────────────────────────────────────────────

    @Test
    fun routesRespond() {
        val port = start {
            before { if (it.path == "/down") halt(503, "Maintenance") else null }
            get("/") { html("<h1>OK</h1>") }
            get("/ping") { text("pong") }
            get("/hello/:name") { json("""{"hi":"${it["name"]}"}""") }
            get("/search") { text("q=${it.queryParams()["q"]}") }
            post("/echo") { respond(200, it.body, mapOf("content-type" to it.contentType)) }
            get("/old") { redirect("/new") }
            get("/forbidden") { halt(403, "Forbidden") }
            get("/boom") { throw RuntimeException("kaboom") }
            notFound { html("Not Found: ${it.path}", 404) }
            onError { json("""{"error":"${it.error}"}""", 500) }
        }

        assertEquals("<h1>OK</h1>", get(port, "/").body())
        assertEquals("pong", get(port, "/ping").body())
        assertContains(get(port, "/hello/Ada").body(), "Ada")
        assertContains(get(port, "/search?q=hello").body(), "hello")
        assertEquals("hello world", post(port, "/echo", "hello world").body())

        val down = get(port, "/down")
        assertEquals(503, down.statusCode())
        assertEquals("Maintenance", down.body())

        val old = get(port, "/old")
        assertEquals(302, old.statusCode())
        assertEquals("/new", old.headers().firstValue("location").orElse(""))

        assertEquals(403, get(port, "/forbidden").statusCode())

        val boom = get(port, "/boom")
        assertEquals(500, boom.statusCode())
        assertContains(boom.body(), "kaboom")

        val missing = get(port, "/missing")
        assertEquals(404, missing.statusCode())
        assertContains(missing.body(), "Not Found: /missing")
    }

    @Test
    fun serverMetadataAndRunning() {
        val app = conduit { get("/") { text("ok") } }
        appHandle = app
        app.serveBackground("127.0.0.1", 0)
        Thread.sleep(150)
        assertTrue(app.localPort() > 0)
        assertTrue(app.running())
        app.stop()
        assertFalse(app.running())
    }

    @Test
    fun settingsReadableBeforeBind() {
        conduit { set("app_name", "Conduit KT") }.use {
            assertEquals("Conduit KT", it.getSetting("app_name"))
        }
    }
}
