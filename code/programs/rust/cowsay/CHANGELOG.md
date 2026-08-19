# Changelog — cowsay

## Unreleased — 2026-08-19 — compiles again under CI

First release built by CI: this program had no `BUILD` file, so nothing ever
compiled it after a merge, and it had silently stopped compiling.

- Set the `wrap` field on the `layout_ir::TextContent` that carries the rendered
  cow. `layout-ir` gained that field in #10345 (Mermaid sequence wrap controls);
  because no `BUILD` file covered this program, the new field was never filled in
  here and `cargo build` had failed with `E0063: missing field 'wrap'` ever since.
  It is set to `false`: a cow is ASCII art whose rows must stay aligned, and the
  scene width is measured from those exact rows, so soft wrapping could only
  shear the balloon off the cow. Hard `\n` breaks are preserved regardless.
