# Changelog

All notable changes to the Java `conduit` package are documented here.

## 0.1.0 — 2026-04-27

Initial release. Java port (WEB09) of the Conduit web framework — the first
JVM port in the series.

### Added

- **DSL** (`Application`): chainable `get`/`post`/`put`/`delete`/`patch`,
  `before`/`after` filters, `notFound`, `onError`, and string settings.
  `AutoCloseable`; binding a `Server` consumes the application.
- **Response helpers** (`Responses`): `html`, `json`, `text`, `respond`,
  `halt`, `redirect` — designed for static import.
- **Request** (`Request`): immutable view with `method`/`path`/`queryString`/
  `body`/`contentType`/`remoteAddr`/`error` accessors and lazily-parsed
  `params`/`queryParams`/`headers` maps (percent-decoded from the JNI wire
  format).
- **Response** (`Response`): status + header map + string body, with
  `headersEncoded()` for the Rust readback.
- **HaltException**: throwable for non-local Sinatra-style halts; detected on
  the Rust side via `IsInstanceOf` and converted to a response (does not route
  to the error handler).
- **ConduitHandler**: `@FunctionalInterface` so handlers are lambdas.
- **Server** (`Server`): `bind`/`serve`/`serveBackground`/`stop`/`localPort`/
  `running`; `AutoCloseable`.
- **Rust cdylib** (`conduit-jni`): cross-thread JNI dispatch via
  `AttachCurrentThreadAsDaemon` + global-ref handlers, `PushLocalFrame`/
  `PopLocalFrame` per request, percent-encoded map marshaling, HTTP status
  clamping (100–599), and CR/LF header-injection defense on both sides. Wraps
  the WEB08 `conduit` facade over `web-core`.
- **`jni-bridge` extensions**: cross-thread callback support — `GetJavaVM`,
  `AttachCurrentThreadAsDaemon`/`DetachCurrentThread`, `New`/`DeleteGlobalRef`,
  `Call{Object,Int,Void}MethodA`, `GetObjectClass`, `IsInstanceOf`,
  `ExceptionOccurred`, and `Push`/`PopLocalFrame`.
- **49 JUnit 5 tests** (Responses, Request, HaltException, Application, and an
  end-to-end Server suite over `java.net.http.HttpClient`) plus 5 pure-Rust
  cdylib unit tests.

### Out of scope

- Binary response bodies (UTF-8 strings only).
- Windows (the cdylib builds, but JVM native-library packaging on Windows is
  untested here).
- A **Kotlin** port (WEB10) reusing this cdylib is tracked separately.
