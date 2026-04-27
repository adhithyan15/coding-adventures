# Changelog

- Initial Rust port of the Nib IR compiler.
- Added function labels, parameter register setup, returns, function calls, and
  wrapping-add masking so the Rust Nib-to-Wasm orchestrator can execute useful
  exported functions through the Wasm runtime.
