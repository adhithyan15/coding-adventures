# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- RFC 1738 URL parsing with single-pass left-to-right algorithm
- `Url` struct with scheme, userinfo, host, port, path, query, fragment fields
- `Url::parse()` for absolute URLs (authority-based and scheme:path forms)
- `Url::resolve()` for RFC 1808 relative URL resolution
- `Url::effective_port()` with defaults for http (80), https (443), ftp (21)
- `Url::authority()` for reconstructing the authority component
- `Url::to_url_string()` and `Display` for serialization
- `percent_encode()` and `percent_decode()` with UTF-8 support
- IPv6 bracket notation support in host parsing
- Scheme validation (`[a-z][a-z0-9+.-]*`)
- Case normalization (scheme and host lowercased)
- Dot-segment removal (`.` and `..`) in path resolution
- 44 unit tests + 4 doc-tests covering all features
