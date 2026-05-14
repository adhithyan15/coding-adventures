# Changelog

## 0.1.0

- Add a compiler from `required_capabilities.json` to generated Rust operation
  source with an HTTP client allowlist derived from `net:dns` and `net:connect`.
- Add a `cli-builder`-backed CLI for compiling the JSON into statically linked
  Rust source.
