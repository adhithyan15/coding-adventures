# UI50: Layout Visual Effects

## Status

Implemented by `layout-effects`, produced by `html-to-layout`, and consumed by
`layout-to-paint` and `html-to-paint`.

## Contract

Visual effects live in `LayoutNode.ext["effects"]` and survive unchanged on the
positioned tree. The typed contract contains a six-value affine transform,
two-dimensional transform origin, clamped opacity, ordered filter chain, blend
mode, and isolation flag. Filter values include blur, drop shadow, brightness,
contrast, saturation, hue rotation, inversion, and opacity.

The contract is deliberately independent from CSS syntax and paint backends.
Producer adapters parse authored values. `layout-effects` validates extension
metadata, composes matrices, resolves box-relative origins, and scales physical
lengths. `layout-to-paint` wraps the complete decoration, content, and descendant
subtree in one `PaintGroup` or `PaintLayer`, preserving isolated opacity and
ordered effects. Hit-testing consumers compose the same transforms.

## Bounded Profile

- CSS mapping supports matrix, translate, scale, rotate, and skew transform
  functions, two-value transform origins, the contract filter set, one shadow
  from each shadow property, standard blend modes, isolation, and a uniform
  border radius.
- Shadow mapping uses a composited-subtree drop-shadow filter rather than the
  full CSS box-shadow silhouette algorithm.
- Transformed link regions are conservative axis-aligned bounds. Exact inverse
  transform containment and rounded descendant clipping remain follow-ups.
- Paint backends may degrade unsupported filters according to their explicit
  capability profiles; layout and scene construction remain deterministic.

## Invariants

- Opacity and bounded percentage filter values are clamped before paint.
- Invalid or unknown extension fields fall back deterministically and can be
  reported without panicking.
- CSS parsing and host toolkit policy never enter the shared effects component.
- Device-pixel-ratio scaling applies to translations, origins, and filter
  lengths exactly once.
- A node's effects wrap its whole subtree, never each child independently.
