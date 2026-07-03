# conduit (Kotlin)

An idiomatic Kotlin DSL for the Conduit web framework — **WEB10** in the series.
Unlike every other port, it ships **no new native code**: it reuses the WEB09
`conduit_jni` cdylib by depending on the Java `conduit` package. Java and Kotlin
share one JVM, the native library loads once, and a Kotlin handler lambda
`{ req -> ... }` is SAM-converted into the Java `ConduitHandler`, so it crosses
the JNI boundary exactly like a Java lambda.

## Quick start

```kotlin
import com.codingadventures.conduitkt.*

val app = conduit {
    before { if (it.path == "/down") halt(503, "Maintenance") else null }
    get("/")            { html("<h1>Hello from Conduit (Kotlin)!</h1>") }
    get("/hello/:name") { json("""{"message":"Hello ${it["name"]}"}""") }
    post("/echo")       { respond(200, it.body, mapOf("content-type" to it.contentType)) }
    notFound            { html("<h1>Not Found: ${it.path}</h1>", 404) }
    onError             { json("""{"error":"Internal Server Error"}""", 500) }
    set("app_name", "Conduit Hello (Kotlin)")
}

app.serve("127.0.0.1", 3000)   // blocks until stopped
```

## DSL

- `conduit { … }` builds an app; `get/post/put/delete/patch(pattern) { … }`,
  `before/after/notFound/onError { … }`, `set(k, v)`.
- Response helpers: `html`, `json`, `text`, `respond`, `halt`, `redirect`.
- `Request` extension properties: `req.method`, `req.path`, `req.body`,
  `req.contentType`, `req.queryString`, `req.error`, and `req["name"]` for
  route params. `param`/`queryParam`/`header`/`params()` carry over from Java.
- `app.serve(host, port)` (blocks), `app.serveBackground(...)`, `app.stop()`,
  `app.localPort()`, `app.running()`; `ConduitApp` is `AutoCloseable`.

## How it fits

```
Kotlin DSL (conduitkt) → Java conduit package (WEB09, unchanged) →
conduit_jni cdylib → conduit (WEB08) → web-core → kqueue/epoll/IOCP
```

All native dispatch, JNI threading, marshaling, and security hardening are
inherited from WEB09.

## Building

```sh
cargo build --manifest-path ../../rust/Cargo.toml -p conduit-jni --release
gradle test
```

The build sets `-Djava.library.path` to the Rust release output and pulls the
Java package in via a Gradle composite build (`includeBuild` in
`settings.gradle.kts`).
