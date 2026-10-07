# Changelog

## 0.1.0 — 2026-10-07

- Add Python 3.12 float expressions and one-argument `print` lowering to IIR.
- Execute the emitted module on Rust `vm-core` with Python-specific division
  and float display hooks.
- Bound text in caller-supplied ASTs and preserve earlier stdout when a later
  VM instruction fails.
