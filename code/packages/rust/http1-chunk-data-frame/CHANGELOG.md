# Changelog

All notable changes to `http1-chunk-data-frame` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-29

### Added

- Added strict borrowed decoding of one caller-bounded, positive-size HTTP/1
  chunk-data frame with an exact trailing CRLF.
- Added payload-free errors and redacted Debug output.

### Security

- Zero-size terminal chunks are rejected so trailer handling cannot be skipped.
- Chunk-size lines, aggregate length, terminal chunks, trailers, transport,
  TLS, credential, audit, trace, and live-provider composition remain absent.
