# paint-vm-raster

Pure Go Paint VM that executes rect-based `PaintScene` values into a `pixel-container`.

This keeps the Go barcode pipeline composable even though Go cannot use the Rust native bridge path in this repo's capability model.
