# barcode-1d

High-level Go pipeline for 1D barcodes.

Pipeline:

`symbology package -> barcode-layout-1d -> PaintScene -> backend-specific Paint VM -> PixelContainer -> PNG`

Backend selection:

- macOS arm64: `paint-vm-metal-native` + `paint-codec-png-native`
- Windows: `paint-vm-gdi-direct` + `paint-codec-png`
- other hosts: `paint-vm-raster` + `paint-codec-png`

The portable `paint-codec-png` path delegates to the repository IC18 PNG core,
so barcode output shares its deterministic RGBA8 encoding and bounded decode
contract. Native platform adapters remain selected only where listed above.
