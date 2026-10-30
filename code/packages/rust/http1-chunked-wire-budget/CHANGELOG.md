# Changelog

All notable changes to `http1-chunked-wire-budget` will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-29

### Added

- Added consumed-and-replaced cumulative chunked wire-byte state.
- Added an exact remaining-budget accessor and stable payload-free failures.

### Security

- Rejects zero-length transitions, wire-length overflow, and cumulative
  encoded-input excess.
- Wire parsing, pre-read buffer bounds, decoded-body and chunk-count policy,
  transport, TLS, credentials, audit, trace, and live-provider composition
  remain absent.
