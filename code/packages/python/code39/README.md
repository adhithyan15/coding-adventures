# code39

Dependency-free Code 39 encoder that emits backend-neutral draw scenes.

## Pipeline

```text
input string
  -> normalize to Code 39 rules
  -> encode symbols
  -> expand to runs
  -> translate runs to DrawScene
```

This package deliberately stops at draw instructions. That keeps the barcode
logic reusable across SVG, PNG, Canvas, or terminal renderers.

## Usage

```python
from code39 import draw_code39
from draw_instructions_svg import render_svg

scene = draw_code39("HELLO-123")
svg = render_svg(scene)
```

## Development

```bash
bash BUILD
```
