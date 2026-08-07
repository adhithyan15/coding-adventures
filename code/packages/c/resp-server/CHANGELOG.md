# Changelog

All notable changes to the `resp-server` (C) package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to semantic versioning.

## [Unreleased]

### Added

- **Initial package — Redis-style RESP server** (CCPP02 port campaign; the first
  protocol server on the stack). Proves `tcp-runtime` (→ `net` + `reactor`) can
  host a real wire protocol by speaking RESP via the `resp-protocol` package.
  - `resp_server_bind` / `resp_server_local_port` / `resp_server_poll` /
    `resp_server_serve` / `resp_server_stop` / `resp_server_destroy` — a thin
    wrapper over `tcp_runtime` whose handler decodes one RESP command per read,
    dispatches it against a shared in-memory keyspace, and writes the encoded
    reply. Reuses `osp_status`.
  - Commands: `PING` (→ `+PONG`, or echo an argument), `ECHO`, `SET`, `GET` (→
    the value or the `$-1` null bulk), and `-ERR unknown command` otherwise.
    Every connection shares one keyspace (the handler's `user` pointer).
  - Test (`tests/resp_server_test.c`): a real RESP round-trip over an actual
    loopback socket (client via `net`, single-threaded via `resp_server_poll`) —
    sends literal RESP frames and asserts the exact reply bytes, including
    overwrite and a null-bulk miss. Verified under ASan+UBSan with 0 leaks.
  - Scope: one command per read chunk (frame reassembly / pipelining across reads
    needs `tcp-runtime`'s stateful-handler follow-up); values over the 8 KiB
    per-read buffer truncate; small command set. Single-threaded on the reactor,
    so the keyspace needs no locking.
