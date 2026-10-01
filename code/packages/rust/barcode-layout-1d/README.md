# barcode-layout-1d

Shared layout crate for linear barcode symbologies in Rust.

This crate owns the reusable seam between symbology logic and paint backends:

```text
symbology rules
  -> Barcode1DRun[]
  -> barcode-layout-1d
  -> PaintScene
  -> paint VM backend
  -> PixelContainer
  -> PNG / other codec
```

It mirrors the role of the TypeScript `barcode-1d` package, but translates
into `PaintScene` so Rust barcodes feed the newer paint pipeline instead of the
legacy draw-instructions stack.

## Portable v1 entry points

`expand_binary_v1`, `expand_width_v1`, `compute_layout_v1`, and
`project_scene_v1` implement the bounded, language-neutral contract in
`code/specs/barcode-layout-1d-v1.md`. They use stable error identifiers,
checked integer geometry, canonical string metadata, and fresh owned results.

The portable scene projector is rectangle-only. Both human-readable text
request forms fail with `human-readable-text-unsupported` before any backend
work. The legacy `layout_barcode_1d` entry point remains unchanged and still
uses native font resolution and shaping when text rendering is enabled.
