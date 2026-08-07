# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `url-parser` crate: a URL parser with the same
  single-pass component split, RFC 1808 relative resolution, and percent coding.
- `url_parse` → a `Url` (scheme, userinfo, host, port, path, query, fragment;
  scheme & host lower-cased, IPv6 brackets kept); `url_resolve` (RFC 1808,
  including `.`/`..` removal); `url_free`.
- Accessors: `url_effective_port` (scheme defaults http=80/https=443/ftp=21),
  `url_authority`, `url_to_string`; percent coding: `url_percent_encode`,
  `url_percent_decode` (validates UTF-8).
- Status-code errors: `URL_ERR_MISSING_SCHEME` / `INVALID_SCHEME` /
  `INVALID_PORT` / `INVALID_PERCENT_ENCODING` / `ALLOC`. On error the output is
  zeroed. Growable buffers use overflow-guarded doubling / checked multiplies.
- Tests use the crate's own cases (component split, lowercasing, `mailto:`,
  ports, authority, errors, percent encode/decode, resolve) under GCC and Clang
  via `iso-harness`.
