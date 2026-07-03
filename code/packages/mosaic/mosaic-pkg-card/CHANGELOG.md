# Changelog

## Unreleased

- Added a light-theme stylesheet (`Card.light.msl`) mirroring the dark theme's structure with a light palette. Selected at build time via `mosaic-compile pkg --theme light` (the style analogue of the layout `--variant`).


## 0.1.0

Initial release. Exports a single Card component built from Box + Column + Text — primitives that are stable in every Mosaic backend today.

This package is intentionally minimal: it's the smallest possible
proof point for the UI29 userland-package architecture, demonstrating
that:

1. A `mosaic-package.toml` parses cleanly
2. .mil/.mll/.msl source files compile end-to-end
3. The component composes from kernel primitives only — no
   bespoke Rust code in any backend

A second userland package (mosaic-pkg-grid, #3647) exercises the
richer kernel surface (For/If/HostTable). Card stays simple to make
the architecture as easy to reason about as possible.
