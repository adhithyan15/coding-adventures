# barcode-1d

High-level Rust pipeline for 1D barcodes.

Pipeline:

`symbology package -> barcode-layout-1d -> PaintScene -> backend-specific Paint VM -> PixelContainer -> PNG`

Supported Rust symbologies in this package:

- Codabar
- Code 128 (Code Set B)
- Code 39
- EAN-13
- ITF
- UPC-A

Backend selection:

- Windows: `paint-vm-direct2d`
- macOS / Apple targets: `paint-metal`
- other hosts: scene construction works, but native pixel rendering is reported as unavailable until a backend is wired
