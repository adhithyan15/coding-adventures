# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- `parse($input)` — single-pass left-to-right URL parser returning a hashref with scheme, userinfo, host, port, path, query, fragment, raw
- `resolve($base_url, $relative)` — RFC 1808 relative URL resolution with dot-segment removal
- `effective_port($url)` — returns explicit port or scheme default (http:80, https:443, ftp:21)
- `authority($url)` — reconstructs the `[userinfo@]host[:port]` authority string
- `to_url_string($url)` — serializes a parsed URL hashref back to a string
- `percent_encode($input)` — percent-encodes non-unreserved characters (uppercase hex)
- `percent_decode($input)` — decodes %XX sequences, interpreting result as UTF-8
- Support for `scheme://authority/path` and `scheme:path` (mailto) URL forms
- IPv6 bracket notation support (`[::1]:8080`)
- Case normalization: scheme and host lowercased, path case preserved
- 46 tests covering parsing, resolution, encoding, edge cases, and error handling
