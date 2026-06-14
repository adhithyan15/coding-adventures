# WEB09 — Java Conduit (JNI port)

## Overview

A Java port of the Conduit web framework, wrapping the Rust `web-core`
engine (via the WEB08 `conduit` facade) through a JNI native library.
Handlers are plain Java lambdas; routing, lifecycle hooks, and HTTP I/O
run in Rust. This is the first JVM port in the Conduit series and the
first to use the `jni-bridge` crate for cross-language dispatch.

Same DSL surface as the Ruby (WEB02), Python (WEB03), Lua (WEB04),
TypeScript (WEB05), Elixir (WEB06/07), and Rust (WEB08) ports — swap the
import, keep the mental model.

```java
import com.codingadventures.conduit.*;
import static com.codingadventures.conduit.Responses.*;

Application app = new Application();

app.before(req -> req.path().equals("/down") ? halt(503, "Maintenance") : null);
app.get("/", req -> html("<h1>Hello from Conduit!</h1>"));
app.get("/hello/:name", req -> json("{\"message\":\"Hello " + req.param("name") + "\"}"));
app.post("/echo", req -> respond(200, req.body(), Map.of("content-type", req.contentType())));
app.notFound(req -> html("<h1>Not Found: " + req.path() + "</h1>", 404));
app.onError(req -> json("{\"error\":\"Internal Server Error\"}", 500));

try (Server server = Server.bind(app, "127.0.0.1", 3000)) {
    server.serve();   // blocks until close()/stop
}
```

---

## Architecture

```
Java DSL (com.codingadventures.conduit.*)
    Application, Server, Request, Response, ConduitHandler, HaltException, Responses
    │  JNI native methods (peer-pointer model: Java holds a long → Rust box)
    ▼
conduit_jni (Rust cdylib, native/conduit_jni)
    nativeNewApp / nativeAddRoute / nativeNewServer / nativeServe / ...
    cross-thread dispatch: AttachCurrentThreadAsDaemon + global-ref handlers
    │  wraps the WEB08 facade
    ▼
conduit (WEB08 Rust facade) → web-core (WebApp, Router, WebServer)
    ▼
embeddable-http-server → tcp-runtime → kqueue/epoll/IOCP
```

### The JNI threading puzzle

The JVM is multi-threaded and `web-core` dispatches HTTP requests on its
own background Rust I/O threads — threads the JVM has never seen. Two JNI
rules make this the central design problem:

1. **`JNIEnv*` is thread-local.** A `JNIEnv` obtained in one thread is
   invalid in another. A Rust I/O thread cannot reuse the `env` from the
   registration call. It must obtain its own via the JavaVM invocation
   interface: `AttachCurrentThread(vm, &env, null)`.

2. **Local references die when the native call returns.** The Java
   handler objects passed to `addRoute` are local refs — invalid after
   `nativeAddRoute` returns. To keep them callable for the server's
   lifetime they must be promoted to **global references**
   (`NewGlobalRef`).

### Solution: capture the JavaVM, pin handlers as globals, attach-on-dispatch

```
                          ┌──────────────────────────────────────────────┐
   Java thread            │  registration (on the JVM thread)            │
   app.get(pat, lambda) → │  NewGlobalRef(lambda) → store in NativeApp    │
                          │  GetJavaVM(env) → cache JavaVM* in DispatchCtx│
                          └──────────────────────────────────────────────┘
                                              │  newServer + serve
                                              ▼
   ┌─ web-core Rust I/O thread (NOT a JVM thread) ───────────────────────┐
   │  request arrives                                                    │
   │  AttachCurrentThreadAsDaemon(vm) → JNIEnv for THIS thread (idempotent)│
   │  build Request jobject (NewObjectA)                                 │
   │  CallObjectMethodA(handlerGlobalRef, handle, [request]) → Response  │
   │  ExceptionCheck → HaltException? error? (see Halt protocol)         │
   │  read Response: status()/body()/headersEncoded()                    │
   │  return WebResponse to web-core                                     │
   └─────────────────────────────────────────────────────────────────────┘
```

`AttachCurrentThreadAsDaemon` is chosen over plain `AttachCurrentThread`
because daemon-attached threads (a) don't need an explicit
`DetachCurrentThread` (web-core's I/O threads are long-lived and pooled,
so attach/detach per request would be wasted work), and (b) don't block
JVM shutdown. Attaching an already-attached thread simply returns the
existing `JNIEnv` — cheap and idempotent.

The dispatch is **concurrent** — each I/O thread attaches independently
and calls Java handlers in parallel. Unlike the Lua port (which
serializes everything through one `lua_State` lock), the JVM is
thread-safe, so we let the BEAM-style/Node-style concurrency through.
Global refs, `jclass` (held as global refs), and `jmethodID` are all
valid across threads, which makes this safe.

---

## Marshaling — percent-encoded key/value strings

Building Java `HashMap` objects from Rust via JNI (FindClass, NewObject,
N× `put`) is chatty and needs object-array machinery `jni-bridge`
doesn't have. Instead we pass each map as a single
percent-encoded `k=v&k2=v2` string and let the other side parse lazily:

| Direction | Field | Encoding |
|-----------|-------|----------|
| Rust → Java | `routeParamsEnc` | pct `k=v&…` (route captures) |
| Rust → Java | `headersEnc` | pct `k=v&…` (lowercase header names) |
| Rust → Java | `queryString` | raw (already `k=v&…` from the client) |
| Java → Rust | `Response.headersEncoded()` | pct `k=v&…` |

This needs exactly one tiny percent-encoder + one decoder per side, and
**zero** new object/array JNI calls beyond `NewStringUTF`. The
percent-encoding (everything outside `[A-Za-z0-9-_.~]` → `%XX`) is
unambiguous and Java decodes it with `java.net.URLDecoder` (`%20` for
space; literal `+` encoded as `%2B` so URLDecoder's `+`→space rule
can't corrupt values).

### Request object

```java
Request(String method, String path, String queryString, String body,
        String contentType, String remoteAddr,
        String routeParamsEnc, String headersEnc, String error)
```

Accessors: `method()`, `path()`, `queryString()`, `body()`,
`contentType()`, `remoteAddr()`, `error()`, `param(name)`,
`queryParam(name)`, `header(name)`, and the lazily-parsed maps
`params()`, `queryParams()`, `headers()`. The `error` field is empty
except for error-handler dispatch (mirrors `conduit.error` in WEB06).

### Response object

```java
new Response(int status, Map<String,String> headers, String body)
```

Read back by Rust via three getters: `int status()`, `String body()`,
`String headersEncoded()`. Header names are lowercased and any value
containing CR/LF is dropped (defense-in-depth against response
splitting; the Rust side re-validates).

---

## Halt protocol

Two ways to short-circuit, matching the sibling ports:

1. **Return a halt Response.** `Responses.halt(status, body)` and
   `redirect(location)` return ordinary `Response` objects. A
   before-filter returning one short-circuits; a handler returning one
   responds directly. This is the common path and needs no exception
   machinery.

2. **Throw `HaltException`.** For deep, non-local halts (Sinatra-style
   `halt()` from inside a helper). After `CallObjectMethodA` the Rust
   dispatch runs `ExceptionCheck`; if set, it grabs the throwable
   (`ExceptionOccurred`), clears it (`ExceptionClear`), and uses
   `IsInstanceOf(throwable, HaltException)`:
   - **HaltException** → read `status()` / `body()` / `headersEncoded()`,
     build the response.
   - **any other Throwable** → read `getMessage()`, route to the
     registered `onError` handler (or a default 500).

A before-filter that returns `null` (Java null) → `None` → continue to
routing.

---

## Native method surface (`com.codingadventures.conduit.Native`)

| Native method | Returns | Purpose |
|---------------|---------|---------|
| `nativeNewApp()` | `long` | Allocate a Rust `NativeApp`, return peer pointer |
| `nativeAddRoute(app, method, pattern, handler)` | `void` | Pin handler as global ref, register closure |
| `nativeAddBefore(app, handler)` | `void` | before filter |
| `nativeAddAfter(app, handler)` | `void` | after filter (response-transforming) |
| `nativeSetNotFound(app, handler)` | `void` | not-found handler |
| `nativeSetErrorHandler(app, handler)` | `void` | error handler (reads `req.error()`) |
| `nativeSetSetting(app, key, value)` | `void` | settings |
| `nativeGetSetting(app, key)` | `String` | settings (or null) |
| `nativeNewServer(app, host, port, maxConn)` | `long` | Consume app, bind socket, return server pointer |
| `nativeServe(server)` | `void` | Block serving until stop |
| `nativeServeBackground(server)` | `void` | Spawn a Rust thread serving; return immediately |
| `nativeStop(server)` | `void` | Signal stop |
| `nativeLocalPort(server)` | `int` | Bound port (for port 0) |
| `nativeRunning(server)` | `boolean` | Is the server thread active |
| `nativeDisposeServer(server)` | `void` | DeleteGlobalRef all handlers, free the box |

The peer pointers are `Box::into_raw(...) as jlong`. `Application` and
`Server` implement `AutoCloseable`; `close()` calls
`nativeDisposeServer` so global refs are released and the box freed.

---

## Required additions to `jni-bridge`

`jni-bridge` currently covers FindClass, NewObjectA, GetMethodID,
GetFieldID, NewStringUTF, GetStringUTFChars, exception clear/check, and
the double-array helpers — enough for the synchronous silicon NIF, but
**not** cross-thread callbacks. WEB09 adds:

**JNIEnv table functions** (offsets are JNI-spec stable; `ExceptionCheck
= 228` in the existing file confirms the layout):

```
NewGlobalRef        = 21    DeleteGlobalRef    = 22
ExceptionOccurred   = 15    GetObjectClass     = 31
IsInstanceOf        = 32    CallObjectMethodA  = 36
CallIntMethodA      = 51    CallVoidMethodA    = 63
GetJavaVM           = 219
```

**JavaVM invocation-interface functions** (a *different* table —
`JavaVM = *const *const c_void` pointing at `JNIInvokeInterface_`):

```
AttachCurrentThread          = 4
DetachCurrentThread          = 5
GetEnv                       = 6
AttachCurrentThreadAsDaemon  = 7
```

New safe wrappers: `jni_new_global_ref`, `jni_delete_global_ref`,
`jni_exception_occurred`, `jni_get_object_class`, `jni_is_instance_of`,
`jni_call_object_method_a`, `jni_call_int_method_a`,
`jni_call_void_method_a`, `jni_get_java_vm`,
`jni_attach_current_thread_as_daemon`, `jni_detach_current_thread`.

All existing `jni-bridge` tests must still pass (additive change only).

---

## Package layout

```
code/packages/java/conduit/
├── BUILD                                  # cargo build + gradle test
├── BUILD_windows
├── CHANGELOG.md
├── README.md
├── build.gradle.kts                       # Java 21, JUnit 5
├── settings.gradle.kts
├── required_capabilities.json             # ["rust", "java", "cargo"]
├── native/conduit_jni/
│   ├── Cargo.toml                         # deps: jni-bridge, conduit (WEB08), web-core
│   └── src/lib.rs                         # the JNI cdylib
└── src/
    ├── main/java/com/codingadventures/conduit/
    │   ├── Application.java               # route/hook/setting registration
    │   ├── Server.java                    # bind/serve/serveBackground/stop, AutoCloseable
    │   ├── Request.java                   # immutable request view, lazy maps
    │   ├── Response.java                  # status/headers/body + headersEncoded()
    │   ├── ConduitHandler.java            # @FunctionalInterface Response handle(Request)
    │   ├── HaltException.java             # RuntimeException carrying status/body/headers
    │   ├── Responses.java                 # static html/json/text/respond/halt/redirect
    │   └── Native.java                    # native-method declarations + library loader
    └── test/java/com/codingadventures/conduit/
        ├── ResponsesTest.java
        ├── RequestTest.java
        ├── HaltExceptionTest.java
        ├── ApplicationTest.java
        └── ServerTest.java                # E2E via java.net.http.HttpClient

code/programs/java/conduit-hello/          # 8-route demo + integration tests
```

---

## Tests (target: 40+)

- `ResponsesTest` — html/json/text/respond/halt/redirect shapes, headers
- `RequestTest` — accessors, lazy map parsing, percent-decoding, error()
- `HaltExceptionTest` — status/body/headers storage, thrown semantics
- `ApplicationTest` — route/filter/handler/setting registration, chaining
- `ServerTest` (E2E) — bind on port 0, real HTTP via `HttpClient`:
  GET `/`, `/hello/:name`, POST `/echo`, before-filter halt(503),
  redirect(302), notFound(404), onError(500), query params, server
  metadata (localPort, running).

`conduit-hello` mirrors the other ports' 8-route demo with 15+
integration tests.

---

## Build

```sh
# native cdylib
cd native/conduit_jni && cargo build --release
# copy libconduit_jni.{dylib,so} → build/native/ on java.library.path
# gradle test with -Djava.library.path=<dir>
./gradlew test
```

The Java side loads the library with `System.load(absolutePath)` (not
`System.loadLibrary`) so the BUILD can point at the cargo output
directory without installing into the system library path.

---

## Out of scope

- **Binary response bodies** — bodies are UTF-8 Strings, matching the
  Lua/TS/Python ports.
- **Kotlin** — a follow-up (WEB10) reuses this exact Rust cdylib; only a
  thin Kotlin-idiomatic wrapper differs. Tracked separately.
- **chunked Transfer-Encoding, HTTP/2, TLS** — owned by `web-core`/out of
  scope as in the other ports.

---

## Future work

- **WEB10 Kotlin** — reuse `conduit_jni` unchanged; Kotlin wrapper with
  idiomatic DSL (trailing-lambda routes).
- **Swift port** — via `objc-bridge`/`c-bridge` (C function pointers).
- **C++ port** — via `c-bridge` (direct C ABI).
