# conduit-hello (Perl)

A complete Sinatra-style demo built on
[`CodingAdventures::Conduit`](../../../packages/perl/conduit/README.md) — the
Perl port of the Conduit web framework over the Rust web-core engine.

## Run

```sh
perl hello.pl          # binds 127.0.0.1:3000
perl hello.pl 8080     # or choose a port
```

The script prints the bound URL once it is listening (it runs the server in the
foreground; press Ctrl-C to stop).

## Routes

| Route | What it shows |
| ----- | ------------- |
| `GET /`               | an HTML greeting (`html`) |
| `GET /hello/:name`    | a route param echoed into JSON (`json`, `param`) |
| `POST /echo`          | request-body echo with content-type passthrough (`respond`, `body`) |
| `GET /search?q=...`   | reading a query param (`query_param`) |
| `GET /redirect`       | a 301 to `/` (`redirect`) |
| `GET /halt`           | a 403 via `halt()` |
| `GET /down`           | a 503 from a `before` filter that short-circuits |
| `GET /error`          | a dying handler routed to the custom `on_error` (500) |
| anything else         | the custom `not_found` handler (404) |

It also wires an `after` filter that logs each request to stderr.

## Try it

```sh
curl http://127.0.0.1:3000/
curl http://127.0.0.1:3000/hello/Adhithya
curl -X POST --data-binary 'ping-pong' http://127.0.0.1:3000/echo
curl -i http://127.0.0.1:3000/redirect
curl -i http://127.0.0.1:3000/down
```

## Test

```sh
prove -v t/
```

`t/smoke.t` launches the demo on an OS-assigned port, hits a few routes, and
asserts on the responses (with an `alarm()` hang guard).
