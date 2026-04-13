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
