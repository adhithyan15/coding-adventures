# Changelog

All notable changes to the `http-server` (C) package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to semantic versioning.

## [Unreleased]

### Added

- **Initial package — HTTP/1.1 server** (CCPP02 port campaign; the second
  protocol server on the stack). Proves `tcp-runtime` (→ `net` + `reactor`) can
  host HTTP: a handler parses the request line + headers and interprets them with
  the `http-core` package.
  - `http_server_bind` / `http_server_local_port` / `http_server_poll` /
    `http_server_serve` / `http_server_stop` / `http_server_destroy` — a thin
    wrapper over `tcp_runtime`. Reuses `osp_status`.
  - Routes: `GET /` → `hello…`; `GET /echo?msg=X` → the `msg` query value (via
    `http-core`'s query parsing); `GET /headers` → the request headers; other
    path → `404`; non-GET → `405`; malformed/oversized → `400`. Every response is
    `Connection: close` (one request/response per connection).
  - Uses `http-core` for the syntax-level interpretation (version parse,
    path/query splitting via `http_request_head_path` /
    `http_request_head_query_value`, header lookup). The byte-level request
    framing is a small in-server parser — the role a standalone `http1` wire crate
    would fill (a future package).
  - Defensive parser: request must arrive whole in one read and be under the 8
    KiB per-read buffer, or `400` — no unbounded reads, no reassembly.
  - Test (`tests/http_server_test.c`): a real HTTP round-trip over an actual
    loopback socket (client via `net`, single-threaded via `http_server_poll`) —
    raw requests asserting status line + body for `GET /`, `/echo?msg=pong`,
    `/headers`, and `404`/`405`/`400`. Verified under ASan+UBSan with 0 leaks.
  - Scope: GET only, one request per read, no body/chunked/keep-alive — each a
    follow-up.
