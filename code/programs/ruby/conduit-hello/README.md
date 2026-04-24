# conduit-hello

A minimal Sinatra-style demo that shows what building on top of `conduit` looks like.

## What it does

Starts an HTTP server on port 3000 and handles two routes:

| Route | Response |
|-------|----------|
| `GET /` | `Hello from Conduit!` |
| `GET /hello/:name` | `Hello <name>` |

Any other path returns `404 Not Found`.

## How it fits in the stack

```
hello.rb  (you are here — 30 lines of Ruby)
    ↓
coding_adventures_conduit  (Ruby DSL layer)
    ↓
conduit_native  (Rust extension — routing lives here via web-core)
    ↓
web-core  (WebApp, WebServer, Router, HookRegistry)
    ↓
embeddable-http-server → tcp-runtime → kqueue/epoll/IOCP
```

Routing is handled entirely by the Rust `WebApp`. When a route matches,
Rust calls back into Ruby with a pre-built env hash and the route index.
Ruby only executes the handler block and returns `[status, headers, body]`.

## Running

```sh
ruby hello.rb
```

Then open <http://localhost:3000/hello/Adhithya> in a browser.

Press `Ctrl-C` to stop.
