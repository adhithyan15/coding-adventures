package com.codingadventures.conduitkt

import java.net.URI
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import java.time.Duration
import java.util.concurrent.TimeUnit
import kotlin.test.AfterTest
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertContains
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import org.junit.jupiter.api.Timeout

/**
 * Integration tests for the bundled 8-route demo ([demoApp]) over real HTTP —
 * the Kotlin equivalent of the other ports' `conduit-hello` test suites.
 */
@Timeout(value = 30, unit = TimeUnit.SECONDS)
class ConduitHelloTest {

    private lateinit var app: ConduitApp
    private var port = 0
    private val client = HttpClient.newBuilder()
        .followRedirects(HttpClient.Redirect.NEVER)
        .connectTimeout(Duration.ofSeconds(5))
        .build()

    @BeforeTest
    fun start() {
        app = demoApp()
        app.serveBackground("127.0.0.1", 0)
        Thread.sleep(150)
        port = app.localPort()
    }

    @AfterTest
    fun stop() {
        app.stop()
        app.close()
    }

    private fun get(path: String): HttpResponse<String> =
        client.send(
            HttpRequest.newBuilder(URI.create("http://127.0.0.1:$port$path"))
                .timeout(Duration.ofSeconds(5)).GET().build(),
            HttpResponse.BodyHandlers.ofString(),
        )

    private fun post(path: String, body: String): HttpResponse<String> =
        client.send(
            HttpRequest.newBuilder(URI.create("http://127.0.0.1:$port$path"))
                .timeout(Duration.ofSeconds(5))
                .header("content-type", "text/plain")
                .POST(HttpRequest.BodyPublishers.ofString(body)).build(),
            HttpResponse.BodyHandlers.ofString(),
        )

    @Test fun root() {
        val r = get("/")
        assertEquals(200, r.statusCode())
        assertContains(r.body(), "Hello from Conduit")
        assertContains(r.body(), "/hello/Adhithya")
    }

    @Test fun helloParam() = assertContains(get("/hello/Adhithya").body(), "Hello Adhithya")

    @Test fun echo() {
        val r = post("/echo", "ping=pong")
        assertEquals(200, r.statusCode())
        assertEquals("ping=pong", r.body())
    }

    @Test fun echoEmpty() = assertEquals("", post("/echo", "").body())

    @Test fun redirect() {
        val r = get("/redirect")
        assertEquals(301, r.statusCode())
        assertEquals("/", r.headers().firstValue("location").orElse(""))
    }

    @Test fun halt() {
        val r = get("/halt")
        assertEquals(403, r.statusCode())
        assertEquals("Forbidden", r.body())
    }

    @Test fun down() {
        val r = get("/down")
        assertEquals(503, r.statusCode())
        assertEquals("Under maintenance", r.body())
    }

    @Test fun error() {
        val r = get("/error")
        assertEquals(500, r.statusCode())
        assertContains(r.body(), "Internal Server Error")
        assertContains(r.body(), "Something went wrong")
    }

    @Test fun missing() {
        val r = get("/missing")
        assertEquals(404, r.statusCode())
        assertContains(r.body(), "Not Found: /missing")
    }

    @Test fun anyUnknownIs404() = assertEquals(404, get("/a/b/c").statusCode())

    @Test fun localPortAssigned() = assertTrue(port > 0)

    @Test fun appNameSetting() {
        demoApp().use { assertEquals("Conduit Hello (Kotlin)", it.getSetting("app_name")) }
    }
}
