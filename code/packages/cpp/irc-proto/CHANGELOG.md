# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C++17 header-only port of the Rust `irc-proto` crate, in
  namespace `ca::irc`: RFC 1459 IRC message parsing and serialization.
- `Message` value type (`std::optional<std::string> prefix`, `std::string
  command`, `std::vector<std::string> params`).
- `parse` — parses an optional `:prefix`, an ASCII-uppercased command, and up to
  15 parameters (the trailing `:param` absorbing the rest of the line); throws
  `ca::irc::ParseError` on malformed input. `try_parse` returns `std::optional`.
- `serialize` — renders a message to a CRLF-terminated `std::string`,
  reintroducing the trailing parameter's `:` when it contains a space, is empty,
  or begins with `:`.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): prefix/command/param
  parsing, the trailing-param rule, the 15-parameter cap, error cases (throwing
  and `try_parse`), serialization, and round-trips.
