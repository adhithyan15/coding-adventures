# WEB10 — Kotlin Conduit (JVM, reuses the Java cdylib)

## Overview

A Kotlin port of the Conduit web framework. Unlike every other port, WEB10
ships **no new Rust** — it reuses the WEB09 `conduit_jni` cdylib unchanged by
depending on the Java `conduit` package. The Kotlin layer is a thin,
idiomatic DSL: trailing-lambda routes, extension properties on `Request`, and
top-level response helpers.

This is possible because Java and Kotlin share one JVM. The native library is
loaded once (by the Java `Native` class's static initializer); a Kotlin
lambda `{ req -> ... }` is **SAM-converted** by the Kotlin compiler into the
Java `ConduitHandler` functional interface, so it crosses the JNI boundary
exactly like a Java lambda.

```kotlin
import com.codingadventures.conduitkt.*

val app = conduit {
    before { if (it.path == "/down") halt(503, "Maintenance") else null }
    get("/")            { html("<h1>Hello from Conduit (Kotlin)!</h1>") }
    get("/hello/:name") { json("""{"message":"Hello ${it.param("name")}"}""") }
    post("/echo")       { respond(200, it.body, mapOf("content-type" to it.contentType)) }
    notFound            { html("<h1>Not Found: ${it.path}</h1>", 404) }
    onError             { json("""{"error":"Internal Server Error"}""", 500) }
    set("app_name", "Conduit Hello (Kotlin)")
}

app.serve("127.0.0.1", 3000)   // blocks until stopped
```

## Why no new cdylib?

The `conduit_jni` symbols are named `Java_com_codingadventures_conduit_Native_*`
— tied to the Java `Native` class. Giving Kotlin its own native methods would
require either recompiling the cdylib with Kotlin-specific JNI symbol names or
a second cdylib. Both contradict "reuse unchanged." Instead, the Kotlin
package **depends on** `com.codingadventures:conduit` (the Java package) and
drives its `Application`/`Server` through a Kotlin-friendly surface. The
native dispatch, threading, marshaling, and security hardening are all
inherited from WEB09.

## Architecture

```
Kotlin DSL (com.codingadventures.conduitkt)
    conduit { }, get/post/..., extension props, html/json/text/halt/redirect
    │  (Kotlin SAM conversion → Java ConduitHandler)
    ▼
Java conduit package (Application, Server, Request, Response, Native)   ← WEB09, unchanged
    ▼
conduit_jni cdylib → conduit (WEB08) → web-core → … → kqueue/epoll/IOCP
```

## DSL surface

| Kotlin | Delegates to |
|--------|--------------|
| `conduit { … }` | builds a Java `Application` via a `ConduitBuilder` receiver |
| `get/post/put/delete/patch(pattern) { handler }` | `Application.get(...)` etc. (trailing lambda) |
| `before { } / after { } / notFound { } / onError { }` | the Java filters/handlers |
| `set(key, value)` / `getSetting(key)` | `Application.set` / `getSetting` |
| `app.serve(host, port)` / `app.serveBackground(...)` / `app.stop()` | wraps `Server.bind` + serve |
| `html/json/text(body, status=200)` | `Responses.html/json/text` |
| `respond(status, body, headers)` | `Responses.respond` |
| `halt(status, body)` / `redirect(location, status=302)` | `Responses.halt/redirect` |

### Ergonomic extensions on `Request`

The Java accessors are methods (`path()`, `method()`, `body()`, …). Kotlin
extension properties expose them as properties — `req.path`, `req.method`,
`req.body`, `req.contentType`, `req.queryString`, `req.error` — plus a
`req["name"]` operator for route params. `param`, `queryParam`, `header`, and
the maps remain as-is (already Kotlin-friendly).

### `Server` lifecycle

`conduit { }` returns a small `ConduitApp` holding the Java `Application`.
`ConduitApp.serve(host, port)` binds a `Server`, serves (blocking);
`serveBackground` returns the `Server` for tests; `localPort`, `running`, and
`stop`/`close` delegate. `ConduitApp` is `AutoCloseable`.

## Package layout

```
code/packages/kotlin/conduit/
├── BUILD                 # cargo build conduit-jni (idempotent) + gradle test
├── BUILD_windows
├── CHANGELOG.md
├── README.md
├── build.gradle.kts      # kotlin("jvm") 2.1.20, Java 21; depends on com.codingadventures:conduit
├── settings.gradle.kts   # includeBuild("../../java/conduit")
├── required_capabilities.json   # ["rust","kotlin","java","cargo"]
└── src/
    ├── main/kotlin/com/codingadventures/conduitkt/Conduit.kt        # DSL + helpers
    ├── main/kotlin/com/codingadventures/conduitkt/ConduitHello.kt   # bundled 8-route demo (gradle run)
    └── test/kotlin/com/codingadventures/conduitkt/
        ├── ConduitKtTest.kt        # DSL + E2E
        └── ConduitHelloTest.kt     # demo integration tests
```

### Deviation: the demo is bundled, not a separate program

The other ports ship a standalone `conduit-hello` program. Kotlin can't: the
Kotlin package is itself a Gradle **composite build** over the Java `conduit`
package, and a separate `conduit-hello` build would transitively include both
`java/conduit` and `kotlin/conduit` — two directories named `conduit`, which
collide on Gradle's directory-derived build path (`:conduit`). So the canonical
8-route demo (`ConduitHello.kt`, `gradle run`) and its integration tests live
inside the package instead. Functionally identical; one fewer Gradle build.

The native library path (`-Djava.library.path=<rust/target/release>`) is set
in `build.gradle.kts` exactly as in the Java package, so
`System.loadLibrary("conduit_jni")` resolves.

## Tests (target: 30+)

- DSL unit tests (no server): builder registers routes/filters/settings;
  response helpers; `Request` extension properties (constructed via the Java
  package-private path is not accessible from Kotlin, so these are covered via
  the E2E suite instead).
- E2E suite via `java.net.http.HttpClient` over a real server on port 0,
  mirroring WEB09's `ServerTest`: `/`, `/hello/:name`, POST `/echo`,
  before-filter halt(503), redirect(302), notFound(404), onError(500), query
  params, server metadata. Class-level `@Timeout(30s)` (same hardening as
  WEB09).
- `conduit-hello`: 8-route demo + integration tests.

## Out of scope

- New Rust / cdylib changes — entirely inherited from WEB09.
- Binary bodies (UTF-8 strings, as in every port).

## Future

The same JVM-reuse trick means a Scala or Clojure port would also be a thin
wrapper over the Java cdylib, should those languages join the repo.
