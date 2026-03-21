# Changelog

All notable changes to the directed-graph-wasm package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

### Added
- wasm-bindgen wrapper around the Rust `directed-graph` crate
- `DirectedGraph` class with full API (camelCase JS convention)
- All algorithms: topologicalSort, hasCycle, transitiveClosure, affectedNodes, independentGroups
- serde-wasm-bindgen for complex type conversion (arrays, nested arrays)
- Unit tests via `cargo test`
- Supports all wasm-pack targets: web, nodejs, bundler
