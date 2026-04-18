# Changelog

## 0.1.0

- add the initial pure C# `paint-vm-svg` backend
- render the paint IR to standalone SVG strings with filters, gradients, clip paths, glyph runs, and safe image hrefs
- expose reusable `CreateSvgContext()`, `CreateSvgVm()`, `RenderToSvgString()`, and `AssembleSvg()` entry points
