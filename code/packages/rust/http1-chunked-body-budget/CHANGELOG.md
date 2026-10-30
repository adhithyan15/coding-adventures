# Changelog

All notable changes to `http1-chunked-body-budget` will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-29

### Added

- Added consumed-and-replaced cumulative decoded-byte and non-terminal
  chunk-count state.
- Added exact remaining-budget accessors and stable payload-free failures.

### Security

- Rejects zero-sized data transitions, count-limit excess, decoded-length
  overflow, and aggregate decoded-body excess.
- Wire framing, terminal and trailer handling, encoded-input bounds,
  transport, TLS, credentials, audit, trace, and live-provider composition
  remain absent.
