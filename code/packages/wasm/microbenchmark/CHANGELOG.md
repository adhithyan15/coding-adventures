# Changelog

## 0.1.0

- Added a bounded `wasm32-unknown-unknown` guest library for paired collection,
  analysis, and strict replay using the shared Rust microbenchmark core.
- Added an owned-buffer ABI, explicit clock/workload imports, and an independent
  Node consumer that loads and exercises the emitted artifact.
