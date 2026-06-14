# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-05-05

### Added

- Rust Conduit demo with the same route set as the other Conduit hello programs.
- `build_app()` helper for constructing the demo application without binding a
  socket, plus route-registration tests.
- Binary entrypoint that binds `127.0.0.1:3000` through `conduit::Server`.
