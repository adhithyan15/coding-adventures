# barcode_1d

High-level Elixir barcode pipeline package.

Current shape:

`text -> code39 -> barcode_layout_1d -> PaintScene -> paint_vm_metal_native -> paint_codec_png_native`

The package chooses a renderer from the current OS and returns PNG bytes.
