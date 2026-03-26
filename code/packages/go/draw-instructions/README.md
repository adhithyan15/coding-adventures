# draw-instructions

Backend-neutral 2D draw instructions for reusable scene generation.

This package is the seam between producer packages and renderer packages.
A barcode package can emit rectangles and text without knowing SVG syntax,
and an SVG package can serialize those instructions without knowing barcode
rules.

## Development

```bash
bash BUILD
```
