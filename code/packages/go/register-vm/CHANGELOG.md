# Changelog

## [0.1.0] - Unreleased

### Added

- Added a Go register VM package with the shared opcode surface used by the Rust, Python, TypeScript, Ruby, and Elixir implementations.
- Implemented accumulator loads, register moves, globals, context slots, arithmetic and bitwise operations, comparisons, relative jumps, object and array literals, property access, deletion, trace capture, and halt/return execution.
- Added inline-cache feedback helpers and lexical context helpers for parity with the complete implementations.
