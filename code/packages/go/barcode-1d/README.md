# barcode-1d

High-level Go pipeline for 1D barcodes.

Pipeline:

`symbology package -> barcode-layout-1d -> PaintScene -> backend-specific Paint VM -> PixelContainer -> PNG`

Backend selection:

- macOS arm64: `paint-vm-metal-native` + `paint-codec-png-native`
- Windows: `paint-vm-gdi-direct` + `paint-codec-png`
- other hosts: `paint-vm-raster` + `paint-codec-png`
