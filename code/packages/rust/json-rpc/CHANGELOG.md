# Changelog — coding-adventures-json-rpc

## [0.1.0] — 2026-04-11

### Added

- `errors` module — `PARSE_ERROR`, `INVALID_REQUEST`, `METHOD_NOT_FOUND`, `INVALID_PARAMS`, `INTERNAL_ERROR` constants; `ResponseError` struct with `Serialize`/`Deserialize` and `std::error::Error`
- `message` module — `Request`, `Response`, `Notification` structs; `Message` enum; `parse_message()` and `message_to_value()` functions; `Serialize`/`Deserialize` impls for `Message`
- `reader` module — `MessageReader<R: BufRead>` with `read_message()` → `Option<Result<Message, ResponseError>>` and `read_raw()`
- `writer` module — `MessageWriter<W: Write>` with `write_message()`, `write_raw()`, and `into_inner()`
- `server` module — `Server<R, W>` with `on_request()`, `on_notification()`, `serve()`; `RequestHandler` and `NotificationHandler` type aliases; panic-safe handler dispatch via `std::panic::catch_unwind`
- `lib.rs` — crate root with re-exports of commonly used types
- 4 unit tests in `server.rs` + 37 integration tests in `tests/integration_tests.rs`
- Dependencies: `serde` 1.x with derive feature, `serde_json` 1.x
