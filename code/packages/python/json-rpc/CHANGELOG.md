# Changelog

All notable changes to `coding-adventures-json-rpc` will be documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-04-11

### Added

- `errors.py` — standard JSON-RPC 2.0 error code constants: `PARSE_ERROR`
  (-32700), `INVALID_REQUEST` (-32600), `METHOD_NOT_FOUND` (-32601),
  `INVALID_PARAMS` (-32602), `INTERNAL_ERROR` (-32603) with full literate-
  programming docstrings explaining when each code applies.

- `message.py` — four message dataclasses (`Request`, `Response`,
  `ResponseError`, `Notification`), `parse_message(raw: str) → Message` to
  convert a raw JSON string to a typed message, and `message_to_dict(msg) →
  dict` for the inverse direction. Discrimination logic: messages with
  `"result"` or `"error"` become `Response`; messages with `"id"` become
  `Request`; otherwise `Notification`.

- `reader.py` — `MessageReader` class that reads Content-Length-framed
  messages from a binary stream. Handles multi-header blocks (ignores
  non-`Content-Length` headers like `Content-Type`), returns `None` on clean
  EOF, raises `JsonRpcError(PARSE_ERROR)` on bad framing or non-UTF-8 payload,
  raises `JsonRpcError(INVALID_REQUEST)` on valid JSON that is not a message.

- `writer.py` — `MessageWriter` class that serializes a typed `Message` to
  compact JSON and writes it with a `Content-Length: <n>\r\n\r\n` header.
  Measures byte length (not character length) to correctly handle multi-byte
  UTF-8 characters such as emoji. Flushes after every message.

- `server.py` — `Server` class that combines reader + writer with a method
  dispatch table. `on_request` and `on_notification` return `self` for
  chaining. `serve()` drives the blocking read-dispatch-write loop until EOF.
  Handlers that return `ResponseError` produce error responses; all other
  return values produce success responses. Handler exceptions are caught and
  converted to `-32603 Internal error`. Unknown requests produce `-32601`;
  unknown notifications are silently ignored per spec.

- `__init__.py` — public API re-exports: all five error constants, all four
  message types, `JsonRpcError`, `parse_message`, `message_to_dict`,
  `MessageReader`, `MessageWriter`, `Server`.

- `tests/test_json_rpc.py` — 40 test cases covering all five areas above,
  including back-to-back message reads, UTF-8 byte-count correctness,
  server dispatch, handler error propagation, and full round-trip tests.
