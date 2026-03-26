# draw-instructions-svg

SVG renderer for backend-neutral draw instructions.

This package knows how to serialize a generic `DrawScene` into SVG. It does not
know what a barcode is. That separation is the whole reason this package
exists.

## Usage

```python
from draw_instructions import create_scene, draw_rect
from draw_instructions_svg import render_svg

scene = create_scene(100, 50, [draw_rect(10, 10, 20, 30)])
svg = render_svg(scene)
```

## Development

```bash
bash BUILD
```
