# Changelog

## 0.1.0

- add the initial pure F# `paint-vm-svg` backend
- render the paint IR to standalone SVG strings with filters, gradients, clip paths, glyph runs, and safe image hrefs
- expose reusable `createSvgContext`, `createSvgVM`, `renderToSvgString`, and `assembleSvg` entry points
