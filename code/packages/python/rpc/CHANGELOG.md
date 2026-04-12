# Changelog — coding-adventures-rpc

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-11

### Added

- `src/rpc/errors.py` — Standard RPC error code constants (`PARSE_ERROR`,
  `INVALID_REQUEST`, `METHOD_NOT_FOUND`, `INVALID_PARAMS`, `INTERNAL_ERROR`)
  and `RpcDecodeError` exception class.
- `src/rpc/message.py` — Codec-agnostic message dataclasses: `RpcRequest`,
  `RpcResponse`, `RpcErrorResponse`, `RpcNotification`; type aliases `RpcId`
  and `RpcMessage`.
- `src/rpc/codec.py` — `RpcCodec` structural protocol (encode/decode); runtime
  guard `check_codec()`.
- `src/rpc/framer.py` — `RpcFramer` structural protocol (read_frame/write_frame);
  runtime guard `check_framer()`.
- `src/rpc/server.py` — `RpcServer[V]` generic class with `on_request()`,
  `on_notification()`, and `serve()` (blocking read-dispatch-write loop with
  `BaseException` panic safety).
- `src/rpc/client.py` — `RpcClient[V]` generic class with `request()`,
  `notify()`, `on_notification()`; `RpcRemoteError` exception for server-side
  errors; monotonically increasing id management starting at 1.
- `src/rpc/__init__.py` — Re-exports entire public API.
- `tests/test_rpc.py` — Comprehensive test suite (8 test groups, 50+ test
  cases) covering messages, error codes, mock doubles, server dispatch, client
  request/notify/push, integration round-trips, and public API surface.
- `pyproject.toml` — Package metadata, ruff lint config, pytest+coverage config.
- `BUILD` / `BUILD_windows` — Build scripts for the monorepo build tool.
- `README.md` — Usage guide, architecture diagram, and implementation notes.
