# conduit (Java)

A Sinatra/Express-inspired web framework for Java, backed by the Rust
`web-core` engine through a JNI native library. This is the **WEB09** port in
the Conduit series — the first JVM port — and uses the zero-dependency
`jni-bridge` crate for the Java ↔ Rust boundary.

Same DSL surface as the Ruby (WEB02), Python (WEB03), Lua (WEB04), TypeScript
(WEB05), Elixir (WEB06/07), and Rust (WEB08) ports.

## Quick start

```java
import com.codingadventures.conduit.*;
import static com.codingadventures.conduit.Responses.*;

try (Application app = new Application()) {
    app.before(req -> req.path().equals("/down") ? halt(503, "Maintenance") : null)
       .get("/", req -> html("<h1>Hello from Conduit!</h1>"))
       .get("/hello/:name", req -> json("{\"message\":\"Hello " + req.param("name") + "\"}"))
       .post("/echo", req -> respond(200, req.body(), java.util.Map.of("content-type", req.contentType())))
       .notFound(req -> html("<h1>Not Found: " + req.path() + "</h1>", 404))
       .onError(req -> json("{\"error\":\"Internal Server Error\"}", 500));

    try (Server server = Server.bind(app, "127.0.0.1", 3000)) {
        server.serve();   // blocks until stop()/close()
    }
}
```

## How it fits in the stack

```
Java DSL (Application, Server, Request, Response, ConduitHandler, HaltException, Responses)
    │  JNI native methods (peer-pointer model: Java holds a long → Rust box)
    ▼
conduit_jni (Rust cdylib)
    cross-thread dispatch: AttachCurrentThreadAsDaemon + global-ref handlers
    ▼
conduit (WEB08 Rust facade) → web-core → embeddable-http-server → tcp-runtime → kqueue/epoll/IOCP
```

## The JNI threading model

`web-core` dispatches HTTP requests on its own background Rust I/O threads —
threads the JVM has never seen. Two JNI rules shape the design:

1. **A `JNIEnv*` is thread-local.** Each I/O thread calls
   `AttachCurrentThreadAsDaemon(vm)` to obtain its own env (idempotent; daemon
   attachment needs no detach and doesn't block JVM shutdown).
2. **Local references die when a native call returns.** Java handler lambdas
   are promoted to **global references** at registration so they stay callable
   for the server's lifetime, and each dispatch is wrapped in a
   `PushLocalFrame`/`PopLocalFrame` pair to keep per-request local refs from
   leaking.

Dispatch is **concurrent** — each I/O thread attaches independently and calls
Java handlers in parallel (the JVM is thread-safe), unlike the Lua port's
single-lock model.

## DSL reference

| Method | Effect |
|--------|--------|
| `app.get/post/put/delete/patch(pattern, handler)` | Register a route |
| `app.before(handler)` | Filter; return a `Response` to short-circuit, `null` to continue |
| `app.after(handler)` | Filter; return a `Response` to replace, `null` to keep |
| `app.notFound(handler)` | Custom 404 handler |
| `app.onError(handler)` | Runs on uncaught exceptions; message via `req.error()` |
| `app.set(key, value)` / `app.getSetting(key)` | Settings |
| `Responses.html/json/text(body[, status])` | Typed responses |
| `Responses.respond(status, body[, headers])` | Custom response |
| `Responses.halt(status, body)` | Return this to short-circuit |
| `Responses.redirect(location[, status])` | `Location` redirect |
| `throw new HaltException(status, body)` | Non-local Sinatra-style halt |

A handler is a `ConduitHandler` — a `@FunctionalInterface` of
`Response handle(Request)`, so handlers are ordinary lambdas.

## Building

```sh
# from this directory:
cargo build --manifest-path ../../rust/Cargo.toml -p conduit-jni --release
gradle test
```

`build.gradle.kts` points `-Djava.library.path` at the Rust release output so
`System.loadLibrary("conduit_jni")` finds the cdylib.

## Notes & limits

- Response bodies are UTF-8 strings (as in the Lua/TS/Python ports); binary
  bodies are out of scope.
- Maps cross the JNI boundary as percent-encoded `k=v&…` strings, decoded with
  `java.net.URLDecoder`.
- A **Kotlin** port (WEB10) will reuse this exact cdylib with an idiomatic
  Kotlin wrapper.
