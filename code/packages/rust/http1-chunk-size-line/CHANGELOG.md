# Changelog

All notable changes to `http1-chunk-size-line` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-29

### Added

- Added strict bounded CRLF chunk-size-line decoding with checked hexadecimal
  accumulation and a caller-owned maximum chunk size.
- Added exact borrowed encoded-line and remainder retention with payload-free
  failures.

### Security

- Chunk extensions are rejected until separately reviewed.
- Full chunk data, aggregate length, terminal-chunk, trailer, transport, TLS,
  credential, audit, trace, and live-provider composition remain unavailable.
