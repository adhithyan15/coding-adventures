# embeddable-http-server

Embeddable HTTP/1 server primitive built on `tcp-runtime`.

This crate owns HTTP/1 request framing and response serialization while leaving
application work behind a handler callback. The eventual language bridges can
wrap this crate and map requests into Rack-style or WSGI-style application
objects without having to reimplement the socket runtime.

Ignored stress coverage binds the native TCP-backed HTTP server and drives
concurrent pipelined requests over real sockets:

```bash
EMBEDDABLE_HTTP_STRESS_CLIENTS=256 \
EMBEDDABLE_HTTP_STRESS_REQUESTS_PER_CLIENT=8 \
  cargo test -p embeddable-http-server \
  native_http_server_handles_pipelined_requests_under_concurrent_load \
  -- --ignored --nocapture
```
