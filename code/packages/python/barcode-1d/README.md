# coding-adventures-barcode-1d

High-level 1D barcode pipeline for Python.

```python
from barcode_1d import render_png

png = render_png("HELLO-123", symbology="code39")
```

Pipeline:

`code39 -> barcode-layout-1d -> PaintScene -> native Paint VM -> PixelContainer -> native PNG codec`
