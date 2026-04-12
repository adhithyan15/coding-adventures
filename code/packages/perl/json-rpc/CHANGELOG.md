# Changelog — CodingAdventures::JsonRpc (Perl)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.01] — 2026-04-11

### Added

- `CodingAdventures::JsonRpc::Errors` — standard error-code constants
  (`PARSE_ERROR`, `INVALID_REQUEST`, `METHOD_NOT_FOUND`, `INVALID_PARAMS`,
  `INTERNAL_ERROR`).
- `CodingAdventures::JsonRpc::Message` — message constructors (`request`,
  `response`, `error_response`, `notification`) plus `parse_message` and
  `classify_message`.
- `CodingAdventures::JsonRpc::Reader` — `MessageReader` class; reads
  Content-Length-framed JSON-RPC messages from a filehandle.
- `CodingAdventures::JsonRpc::Writer` — `MessageWriter` class; writes
  Content-Length-framed messages to a filehandle.
- `CodingAdventures::JsonRpc::Server` — dispatch server; `on_request`,
  `on_notification`, `serve()`.
- `CodingAdventures::JsonRpc` — umbrella module re-exporting all of the above.
- `t/json_rpc.t` — 28 Test2::V0 test cases.

### Notes

- Uses `JSON::PP` (Perl core since 5.14) — no CPAN runtime dependencies.
- All filehandles opened with `binmode($fh, ':raw')` for correct byte counting.
