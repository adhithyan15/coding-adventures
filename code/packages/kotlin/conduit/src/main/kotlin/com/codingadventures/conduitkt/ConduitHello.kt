/**
 * Conduit demo (Kotlin) — the canonical eight-route example, mirroring the
 * Ruby/Python/Lua/TypeScript/Elixir/Rust/Java `conduit-hello` demos.
 *
 * It lives in the framework package (rather than a separate program) because
 * the Kotlin package is itself a Gradle composite build over the Java package,
 * and a second `conduit`-named composite for a standalone demo would collide on
 * Gradle's directory-derived build path. Run it with `gradle run`.
 *
 * ```
 *   GET  /              HTML home
 *   GET  /hello/:name   JSON greeting using a route param
 *   POST /echo          echoes the request body
 *   GET  /redirect      301 to /
 *   GET  /halt          halt(403, "Forbidden")
 *   GET  /down          before-filter halt(503)
 *   GET  /error         throws → routes to the error handler (500)
 *   (any)/missing       custom not-found handler (404)
 * ```
 */
package com.codingadventures.conduitkt

/** Build the demo application. Pure — easy to test. */
fun demoApp(): ConduitApp = conduit {
    before { if (it.path == "/down") halt(503, "Under maintenance") else null }
    get("/") {
        html(
            """
            <!DOCTYPE html>
            <html><head><title>Conduit Hello</title></head>
            <body>
              <h1>Hello from Conduit (Kotlin)!</h1>
              <p>Try <a href="/hello/Adhithya">/hello/Adhithya</a>.</p>
            </body></html>
            """.trimIndent(),
        )
    }
    get("/hello/:name") { json("""{"message":"Hello ${it["name"]}"}""") }
    post("/echo") { respond(200, it.body, mapOf("content-type" to it.contentType)) }
    get("/redirect") { redirect("/", 301) }
    get("/halt") { halt(403, "Forbidden") }
    get("/error") { throw RuntimeException("Something went wrong!") }
    notFound { html("<h1>Not Found: ${it.path}</h1>", 404) }
    onError { json("""{"error":"Internal Server Error","detail":"${it.error}"}""", 500) }
    set("app_name", "Conduit Hello (Kotlin)")
}

fun main(args: Array<String>) {
    var host = "127.0.0.1"
    var port = 3000
    var i = 0
    while (i < args.size - 1) {
        when (args[i]) {
            "--host" -> host = args[i + 1]
            "--port" -> port = args[i + 1].toInt()
        }
        i++
    }
    println("Conduit Hello listening on http://$host:$port")
    demoApp().serve(host, port)
}
