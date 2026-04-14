# barcode-1d

High-level Go pipeline for 1D barcodes.

Pipeline:

`symbology package -> barcode-layout-1d -> PaintScene -> paint-vm-raster -> paint-codec-png`

Go currently uses a pure Go raster backend so the language can render end to end without violating the repo's Go capability restrictions around native FFI.
