# conduit (Go)

A Sinatra/Express-style web framework for Go — the WEB14 port in the
cross-language Conduit family.

Handlers are ordinary Go functions. Routing, lifecycle hooks, and HTTP I/O run
in the Rust `web-core` engine via the reusable `conduit-capi` C ABI. The only
Go-level FFI surface is a cgo preamble; no new trust boundary is introduced
because header-injection defence, status clamping, UTF-8 validation, and panic
isolation are already enforced by the Rust layer.

## Architecture

```
Go handler funcs (your code)
    │  //export trampolines + cgo.Handle registry
    ▼
conduit-capi (Rust staticlib — the C ABI from WEB12)
    ▼
conduit (WEB08 facade) → web-core → embeddable-http-server → tcp-runtime
```

## Installation

```sh
# Requires a compiled conduit-capi static lib in:
#   code/packages/rust/target/release/libconduit_capi.a
cd code/packages/rust/conduit-capi && cargo build --release
```

Then in your `go.mod`:

```
require github.com/adhithyan15/coding-adventures/code/packages/go/conduit v0.0.0

replace github.com/adhithyan15/coding-adventures/code/packages/go/conduit => ../../packages/go/conduit
```

## Usage

```go
package main

import (
    "fmt"
    "log"

    "github.com/adhithyan15/coding-adventures/code/packages/go/conduit"
)

func main() {
    app := conduit.New()

    // Before-filter: runs before every route handler.
    app.Before(func(req *conduit.Request) *conduit.Response {
        if req.Path() == "/down" {
            conduit.Halt(503, "maintenance")
        }
        return nil // continue to route handler
    })

    // After-hook: stamps a header on every response.
    app.After(func(_ *conduit.Request, resp conduit.Response) conduit.Response {
        resp.Headers = append(resp.Headers, conduit.Header{Name: "x-served-by", Value: "conduit-go"})
        return resp
    })

    // Route handlers.
    app.Get("/", func(*conduit.Request) conduit.Response {
        return conduit.HTML("<h1>Hello, Conduit!</h1>")
    })
    app.Get("/hello/:name", func(req *conduit.Request) conduit.Response {
        name, _ := req.Param("name")
        return conduit.JSON(fmt.Sprintf(`{"hi":"%s"}`, name))
    })

    // Bind and serve.
    server, err := app.Bind("0.0.0.0", 3000)
    if err != nil {
        log.Fatal(err)
    }
    fmt.Println("Listening on port", server.LocalPort())
    server.Serve() // blocks
}
```

## API reference

### Response helpers

| Function                            | Status | Content-Type                     |
|-------------------------------------|--------|----------------------------------|
| `HTML(body, status...)`             | 200    | `text/html; charset=utf-8`       |
| `JSON(body, status...)`             | 200    | `application/json`               |
| `Text(body, status...)`             | 200    | `text/plain; charset=utf-8`      |
| `Respond(status, body, headers...)` | any    | (you supply the header)          |
| `Redirect(location, status...)`     | 302    | (sets `location` header)         |

`Redirect` returns `(Response, error)` — it rejects locations containing `\r`
or `\n` to prevent response-splitting attacks.

### Handler types

```go
type HandlerFunc func(*Request) Response
type BeforeFunc  func(*Request) *Response   // nil → continue
type AfterFunc   func(*Request, Response) Response
```

### Application

```go
app := conduit.New()
app.Get("/path", handler)
app.Post("/path", handler)
app.Put("/path", handler)
app.Delete("/path", handler)
app.Patch("/path", handler)
app.Route("METHOD", "/path", handler)   // arbitrary method
app.Before(beforeFn)                    // before-filter
app.After(afterFn)                      // after-hook
app.NotFound(handler)                   // custom 404
app.OnError(handler)                    // custom 500
app.Set("key", "value")                 // application setting
v, ok := app.GetSetting("key")
server, err := app.Bind("host", port)  // consumes app; returns Server
app.Free()                              // free an unbound app
```

### Request accessors

```go
req.Method()      string
req.Path()        string
req.QueryString() string
req.ContentType() string
req.RemoteAddr()  string
req.Error()       string          // non-empty inside OnError handlers
req.Body()        []byte
req.BodyString()  string
req.Param("name")  (string, bool) // :name route parameter
req.Query("name")  (string, bool) // query-string value
req.Header("name") (string, bool) // request header (case-insensitive)
```

### Server

```go
server.Serve()           bool   // foreground (blocks)
server.ServeBackground() bool   // background OS thread
server.Stop()                   // stop the server
server.LocalPort()       uint16 // actual bound port
server.Running()         bool
server.Close()                  // free native resources
```

### Halt

```go
conduit.Halt(status int, body string)
```

Call `Halt` from any handler or before-filter to short-circuit request
processing and return the given status/body immediately. Uses Go's `panic`/
`recover` mechanism internally; the trampoline catches it before it crosses the
C boundary.

## cgo.Handle pattern

Go closures are passed to C via [`cgo.Handle`](https://pkg.go.dev/runtime/cgo#Handle)
(Go 1.21+), a GC-safe integer handle registry. This avoids the `go vet`
"misuse of unsafe.Pointer" warning from naïve `uintptr → void*` casts and
prevents closures from being garbage-collected while C holds a reference.

## Building

```sh
# From the package root:
CGO_ENABLED=1 go test ./... -v -cover
```

Requires `libconduit_capi.a` at
`../../rust/target/release/libconduit_capi.a` (relative to this package).

## Test coverage

14 tests including 8 end-to-end subtests against a live background server with a
30-second watchdog to prevent hangs. Covers route parameters, query strings, POST
bodies, before-halt short-circuiting, after-hook response mutation, error
handling, not-found, and redirects.

## Stack position

```
WEB00 web-core (Rust)
WEB02 conduit-sinatra (Ruby reference)
WEB08 conduit (Rust WEB08 facade)
WEB12 conduit-capi (Rust C ABI — shared by WEB12–WEB18)
  ├── WEB12 conduit (Swift)
  ├── WEB13 conduit (C++)
  └── WEB14 conduit (Go)  ← you are here
```
