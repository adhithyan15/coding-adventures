# Changelog

## Unreleased — bounded Python print calls

- Lower an exact zero-argument `print()` grammar call to a Rust `vm-core`
  builtin that writes one newline and retains it if a later VM instruction
  fails. Other call forms remain bounded by the existing pilot.
- Lower exactly two positional float expressions from the Python grammar tree
  to IIR and print them through a Rust VM builtin with one separating space.
  Preserve source-order evaluation, output limits, and earlier completed
  output if a later argument or statement fails.

## 0.1.0 — 2026-10-07

- Add Python 3.12 float expressions and one-argument `print` lowering to IIR.
- Execute the emitted module on Rust `vm-core` with Python-specific division
  and float display hooks.
- Bound text in caller-supplied ASTs and preserve earlier stdout when a later
  VM instruction fails.
