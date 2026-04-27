# code39

Dependency-free Code 39 encoder that emits backend-neutral paint scenes.

## Pipeline

```text
input string
  -> normalize to Code 39 rules
  -> encode symbols
  -> expand to runs
  -> barcode-layout-1d
  -> PaintScene
```

This package deliberately stops at `PaintScene`. That keeps the barcode logic
reusable across Metal, Direct2D, GDI, Canvas, SVG, or future image codecs.

## Usage

```python
from code39 import draw_code39

scene = draw_code39("HELLO-123")
```

## Development

```bash
bash BUILD
```
