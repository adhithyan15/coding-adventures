# Changelog

All notable changes to the Go `json-rpc` package will be documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-04-11

### Added

- `errors.go` — standard JSON-RPC 2.0 error code constants (`ParseError` -32700,
  `InvalidRequest` -32600, `MethodNotFound` -32601, `InvalidParams` -32602,
  `InternalError` -32603), `ResponseError` struct implementing `error`, and
  `NewResponseError` factory function. Also contains the package-level doc
  comment explaining the full JSON-RPC architecture.

- `message.go` — `Message` interface (discriminated union via `isMessage()` marker),
  `*Request`, `*Response`, and `*Notification` concrete types. `ParseMessage(raw string)`
  converts raw JSON to typed messages using key-set discrimination (no explicit type
  field). `MessageToMap(msg Message)` is the inverse — produces a
  `map[string]interface{}` ready for `json.Marshal`. JSON number IDs arrive as
  `float64` from Go's generic unmarshaler; `normalizeID` converts whole-number
  floats to `int` for cleaner comparisons.

- `reader.go` — `MessageReader` struct wrapping a `bufio.Reader` for efficient
  line-by-line header reading. `NewReader(r io.Reader)`, `ReadRaw() (string, error)`,
  and `ReadMessage() (Message, error)`. Returns `("", io.EOF)` on clean end-of-stream
  between messages. Returns `*ResponseError` on framing errors (wrong Content-Length,
  truncated payload, missing header).

- `writer.go` — `MessageWriter` struct. `NewWriter(w io.Writer)`, `WriteRaw(jsonStr string) error`,
  and `WriteMessage(msg Message) error`. Measures byte length after UTF-8 encoding
  (not character count) to handle emoji and CJK correctly. Uses compact JSON
  (`json.Marshal` without indentation) to minimize payload size.

- `server.go` — `Server` struct with `NewServer(in io.Reader, out io.Writer)`,
  `OnRequest`, `OnNotification` (both chainable), and `Serve()`. Dispatch loop:
  requests → find handler → call → write Response; notifications → find handler
  → call (no response); unknown requests → `-32601`; unknown notifications → silent.
  Handler panics are recovered and converted to `-32603 Internal error` to keep
  the server alive.

- `json_rpc_test.go` — 35 test functions organized into five groups: ParseMessage,
  MessageToMap, MessageReader, MessageWriter, Server, and round-trip tests. Tests
  cover back-to-back messages, UTF-8 byte-count correctness (emoji), server dispatch,
  handler error propagation, and clean EOF handling.
