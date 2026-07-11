# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `url-parser` crate, in namespace
  `ca::url`: a URL parser with the same component split, RFC 1808 relative
  resolution, and percent coding.
- `Url::parse` → a `Url` (with `std::optional` for userinfo/host/port/query/
  fragment; scheme & host lower-cased, IPv6 brackets kept); `Url::resolve` (RFC
  1808, `.`/`..` removal); `effective_port`, `authority`, `to_url_string`.
- Free functions `percent_encode` / `percent_decode` (decode validates UTF-8).
- `parse` / `resolve` / `percent_decode` throw `ca::url::ParseError` (carrying an
  `Error` kind) on failure. Uses `std::string_view` for zero-copy slicing.
- Tests use the crate's own cases (component split, lowercasing, `mailto:`,
  ports, authority, errors, percent encode/decode, resolve) under GCC and Clang
  via `iso-harness`.
