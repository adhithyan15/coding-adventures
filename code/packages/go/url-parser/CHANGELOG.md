# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- RFC 1738 URL parsing (scheme, userinfo, host, port, path, query, fragment)
- Relative URL resolution (RFC 1808)
- Percent-encoding and decoding with UTF-8 support
- Default port lookup (http=80, https=443, ftp=21)
- IPv6 bracket notation support
- Dot-segment removal (RFC 3986 S5.2.4)
- Opaque URI support (mailto:, etc.)
- Optional fields via Go pointer types (nil = absent)
- UrlError struct with Kind classifier
- 45 unit tests
