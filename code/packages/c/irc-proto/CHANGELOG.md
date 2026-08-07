# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `irc-proto` crate: RFC 1459 IRC message
  parsing and serialization with no socket/thread/buffer dependencies.
- `irc_parse` — parses an optional `:prefix`, an ASCII-uppercased command, and
  up to 15 parameters (the trailing `:param` absorbing the rest of the line with
  its `:` stripped); returns a typed `IrcStatus` (empty/whitespace-only line,
  prefix-with-no-command, no-command) where the Rust version returns a `Result`.
- `irc_serialize` — renders a message to CRLF-terminated wire bytes, reintroducing
  the trailing parameter's `:` when it contains a space, is empty, or begins
  with `:`; returns a malloc'd, NUL-terminated buffer.
- `irc_message_free` releases the malloc'd fields of a parse result. Pure-ISO
  string helpers replace POSIX `strdup`/`strndup`; the serializer's growable byte
  buffer guards its doubling against `size_t` overflow.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): prefix/command/param
  parsing, the trailing-param rule, the 15-parameter cap, error cases,
  serialization, and round-trips — mirroring the Rust crate's unit tests.
