# `smart-home-platform-http`: Windows RST-on-close race in test HTTP client — same accepted quirk as the embeddable TCP tests, this time on the client side

`build (windows-latest)` failed with exactly 2 of 54 `smart-home-platform-http`
tests down — `runtime_web_app_serves_runtime_snapshot_over_repo_http_server`
and `runtime_web_app_serves_smoke_script_over_repo_http_server` — both
panicking at the *first* `read_line` call for the HTTP status line:
`Os { code: 10054, kind: ConnectionReset, message: "An existing connection
was forcibly closed by the remote host." }`. The other 52 tests, which hit the
same shared `WebServer::bind_windows` instance through the same `http_request`
test helper with `Connection: close` on every request, passed.

This is the client-side twin of a quirk this very PR already normalized on
the server side (see `embeddable-tcp-server::assert_peer_closed`, which
explicitly treats `io::ErrorKind::ConnectionReset` as an acceptable outcome
of a deliberate peer close): on Windows, closing a socket while there is any
unflushed/unacknowledged data in play can produce a hard RST instead of a
graceful FIN, and it appears more readily on the two endpoints serving the
largest response bodies (a JSON runtime snapshot and a generated shell
script) — plausibly a larger multi-chunk write racing the reactor's own
`Connection: close` teardown. It reproduces only on `windows-latest`; the
identical test passes reliably on Linux and macOS CI.

Rather than touch the Windows reactor's write/close sequencing in
`tcp-runtime`/`embeddable-http-server` (large, shared, security-sensitive
surface, out of scope for this rescue), keep the fix in the test helper. A
fresh-connection retry alone was not sufficient under a later loaded Windows
build: all five attempts for the runtime snapshot could hit the same close
race. The helper now sends `Connection: keep-alive`, reads exactly the
advertised `Content-Length`, and lets dropping the client stream initiate the
close only after the response is complete. It retains the narrow whole-request
retry for `io::ErrorKind::ConnectionReset`; any other error still panics
immediately. This avoids depending on close timing while preserving the known
Windows-platform-quirk handling already established elsewhere in the repo.
