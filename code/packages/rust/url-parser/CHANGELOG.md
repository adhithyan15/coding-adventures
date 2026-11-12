# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added

- Add required_capabilities.json declaring only the capabilities used by #[cfg(test)] modules (the analyzer scans cfg(test) code); production code is pure.

### Fixed

- required_capabilities.json no longer declares a false `fs:list` ("Path::canonicalize resolves fixture paths"). The hits were `Url::canonicalize()` calls on this crate's own type, matched by the analyzer's `.canonicalize()` heuristic. The analyzer now resolves the receiver (`Url::parse(..).unwrap().canonicalize()` reaches the inherent `Url::canonicalize`), and the crate is declared pure (`capabilities: []`).

## [0.2.0] - 2026-08-27

### Added

- `Url::canonicalize()` for stable URL identity across scheme/host case,
  default ports, path dot segments, and equivalent percent escapes.
- Canonicalization validates malformed percent escapes while preserving
  Unicode source text and URL fragments.

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
