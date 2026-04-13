# coding-adventures-paint-vm-metal-native

Rust-backed Metal Paint VM bridge for Python.

This package keeps the boundary intentionally narrow:

- Python owns barcode encoding and `PaintScene` construction.
- The native extension executes rect-only paint instructions through Metal.
- The bridge returns a `PixelContainer`.

## Usage

```python
from paint_instructions import paint_rect, paint_scene
from paint_vm_metal_native import render

scene = paint_scene(
    40,
    20,
    [paint_rect(10, 0, 20, 20, "#000000")],
    "#ffffff",
)
pixels = render(scene)
```
