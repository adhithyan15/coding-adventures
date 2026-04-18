# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-17

### Added

- `Interest` wrapper for readable, writable, edge-triggered, and one-shot flags
- `EpollEvent` token-carrying ready event type
- `Epoll` wrapper with `new`, `add`, `modify`, `delete`, and `wait`
- Linux-only readiness tests and non-Linux unsupported fallback
