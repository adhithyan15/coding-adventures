/**
 * Conduit — idiomatic Kotlin DSL over the Java Conduit web framework (WEB10).
 *
 * Ships no native code: it reuses the WEB09 `conduit_jni` cdylib through the
 * Java `conduit` package. A Kotlin handler lambda `{ req -> ... }` is
 * SAM-converted by the compiler into the Java [ConduitHandler] functional
 * interface, so it crosses the JNI boundary exactly like a Java lambda.
 *
 * ```kotlin
 * import com.codingadventures.conduitkt.*
 *
 * val app = conduit {
 *     before { if (it.path == "/down") halt(503, "Maintenance") else null }
 *     get("/")            { html("<h1>Hello from Conduit (Kotlin)!</h1>") }
 *     get("/hello/:name") { json("""{"message":"Hello ${it.param("name")}"}""") }
 *     post("/echo")       { respond(200, it.body, mapOf("content-type" to it.contentType)) }
 *     notFound            { html("<h1>Not Found: ${it.path}</h1>", 404) }
 *     onError             { json("""{"error":"Internal Server Error"}""", 500) }
 *     set("app_name", "Conduit Hello (Kotlin)")
 * }
 *
 * app.serve("127.0.0.1", 3000)   // blocks until stopped
 * ```
 */
package com.codingadventures.conduitkt

import com.codingadventures.conduit.Application
import com.codingadventures.conduit.ConduitHandler
import com.codingadventures.conduit.Responses
import com.codingadventures.conduit.Server

// Re-export the Java request/response types under Kotlin-friendly names so
// users only import from this package.
typealias Request = com.codingadventures.conduit.Request
typealias Response = com.codingadventures.conduit.Response
typealias HaltException = com.codingadventures.conduit.HaltException

// ── Ergonomic extension properties on Request ───────────────────────────────
//
// The Java accessors are zero-arg methods (path(), method(), …). Kotlin sees
// those as functions; these extension properties expose them as `req.path`.

/** HTTP method, e.g. `"GET"`. */
val Request.method: String get() = method()

/** Request path without the query string. */
val Request.path: String get() = path()

/** Raw query string without the leading `?`. */
val Request.queryString: String get() = queryString()

/** Raw request body as a UTF-8 string. */
val Request.body: String get() = body()

/** Value of the `Content-Type` header, or `""`. */
val Request.contentType: String get() = contentType()

/** Remote peer IP address. */
val Request.remoteAddr: String get() = remoteAddr()

/** Error message — non-empty only inside the error handler. */
val Request.error: String get() = error()

/** Look up a route param with `req["name"]`. Returns `null` if absent. */
operator fun Request.get(name: String): String? = param(name)

// ── Response helpers (top-level, mirror Responses) ──────────────────────────

/** `200 text/html` response (status overridable). */
fun html(body: String, status: Int = 200): Response = Responses.html(body, status)

/** `application/json` response from pre-serialized JSON text. */
fun json(body: String, status: Int = 200): Response = Responses.json(body, status)

/** `text/plain` response. */
fun text(body: String, status: Int = 200): Response = Responses.text(body, status)

/** Arbitrary response with explicit headers. */
fun respond(status: Int, body: String, headers: Map<String, String> = emptyMap()): Response =
    Responses.respond(status, body, headers)

/** A halt response — return it from a filter/handler to short-circuit. */
fun halt(status: Int, body: String): Response = Responses.halt(status, body)

/** A redirect to [location] (default `302`). Rejects CR/LF in the location. */
fun redirect(location: String, status: Int = 302): Response = Responses.redirect(location, status)

// ── DSL builder ─────────────────────────────────────────────────────────────

/**
 * Receiver for the [conduit] builder block. Each method registers on the
 * underlying Java [Application]. Handlers are Kotlin lambdas, SAM-converted to
 * [ConduitHandler].
 */
class ConduitBuilder internal constructor(@PublishedApi internal val app: Application) {
    fun get(pattern: String, handler: (Request) -> Response?) {
        app.get(pattern, ConduitHandler { handler(it) })
    }

    fun post(pattern: String, handler: (Request) -> Response?) {
        app.post(pattern, ConduitHandler { handler(it) })
    }

    fun put(pattern: String, handler: (Request) -> Response?) {
        app.put(pattern, ConduitHandler { handler(it) })
    }

    fun delete(pattern: String, handler: (Request) -> Response?) {
        app.delete(pattern, ConduitHandler { handler(it) })
    }

    fun patch(pattern: String, handler: (Request) -> Response?) {
        app.patch(pattern, ConduitHandler { handler(it) })
    }

    /** Register a route for an arbitrary HTTP method. */
    fun route(method: String, pattern: String, handler: (Request) -> Response?) {
        app.route(method, pattern, ConduitHandler { handler(it) })
    }

    /** Before filter — return a [Response] to short-circuit, `null` to continue. */
    fun before(handler: (Request) -> Response?) {
        app.before(ConduitHandler { handler(it) })
    }

    /** After filter — return a [Response] to replace, `null` to keep the prior one. */
    fun after(handler: (Request) -> Response?) {
        app.after(ConduitHandler { handler(it) })
    }

    /** Custom not-found handler. */
    fun notFound(handler: (Request) -> Response?) {
        app.notFound(ConduitHandler { handler(it) })
    }

    /** Error handler — message available via `req.error`. */
    fun onError(handler: (Request) -> Response?) {
        app.onError(ConduitHandler { handler(it) })
    }

    /** Store a string setting. */
    fun set(key: String, value: String) {
        app.set(key, value)
    }

    /** Read a string setting, or `null`. */
    fun getSetting(key: String): String? = app.getSetting(key)
}

/**
 * Build a [ConduitApp] with the given DSL block.
 *
 * The returned app owns a native peer; bind it with [ConduitApp.serve] /
 * [ConduitApp.serveBackground]. Use it in a `use { }` block (it is
 * [AutoCloseable]) or close it explicitly.
 */
fun conduit(block: ConduitBuilder.() -> Unit): ConduitApp {
    val app = Application()
    ConduitBuilder(app).apply(block)
    return ConduitApp(app)
}

// ── App + server lifecycle ──────────────────────────────────────────────────

/**
 * A built Conduit application. Wraps the Java [Application] and the bound
 * [Server]. Binding happens lazily on the first [serve]/[serveBackground].
 */
class ConduitApp internal constructor(private val application: Application) : AutoCloseable {
    private var server: Server? = null

    /**
     * Read a setting registered in the builder. Valid before the app is bound
     * to a server (binding consumes the underlying Java application).
     */
    fun getSetting(key: String): String? = application.getSetting(key)

    /** Bind to [host]:[port] and serve on the calling thread (blocks until stopped). */
    fun serve(host: String = "127.0.0.1", port: Int = 3000) {
        bind(host, port).serve()
    }

    /** Bind and serve on a background thread; returns this for chaining/tests. */
    fun serveBackground(host: String = "127.0.0.1", port: Int = 0): ConduitApp {
        bind(host, port).serveBackground()
        return this
    }

    /** The bound TCP port (valid after a serve/serveBackground call). */
    fun localPort(): Int = requireServer().localPort()

    /** Whether the server thread is active. */
    fun running(): Boolean = server?.running() ?: false

    /** Signal the server to stop. */
    fun stop() {
        server?.stop()
    }

    private fun bind(host: String, port: Int): Server {
        check(server == null) { "server already bound" }
        return Server.bind(application, host, port).also { server = it }
    }

    private fun requireServer(): Server =
        server ?: error("server not bound — call serve()/serveBackground() first")

    override fun close() {
        server?.close()
        // If never bound, the Java Application still owns a native peer; close it.
        if (server == null) application.close()
        server = null
    }
}
