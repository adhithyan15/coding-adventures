# UI51: Layered Background Painting

## Status

Implemented by `layout-backgrounds`, produced by `html-to-layout`, and consumed
by `layout-to-paint` through backend-neutral paint instructions.

## Contract

`LayoutNode.ext["backgrounds"]` stores an ordered list of image, linear-gradient,
and radial-gradient layers. Every layer independently carries two-axis position,
size, repeat, origin, and clip values. The same contract records border and
padding insets plus four elliptical corner-radius pairs so consumers can resolve
border, padding, and content painting boxes after final layout.

The component owns serialization, tolerant decoding, radius normalization, and
painting-box geometry. It contains no CSS, HTML, resource transport, paint
backend, or host-toolkit policy. Producer adapters parse authored syntax;
painting consumers emit ordinary gradients, paths, images, and clips.

## Invariants

- Layers retain CSS front-to-back order; painters emit them back-to-front above
  the background color and below borders and content.
- Corner radii are proportionally reduced when opposing sums exceed box size.
- Percentage positions resolve against remaining space after tile sizing.
- Repeat expansion is bounded before entering the paint scene.
- Background colors honor the final layer's clip box, and nested clip boxes
  reduce each elliptical radius by its corresponding border/padding inset.
- Invalid extension fields fall back deterministically and produce diagnostics.

## Bounded Profile

The initial CSS adapter supports URL images, two-color-or-greater linear and
radial gradients, explicit/percentage sizing and positioning, axis repeat,
painting boxes, and four-value elliptical radii. Gradient shape keywords,
intrinsic-resource `cover`/`contain`, rounded image masks, and full border-style
geometry remain explicit follow-ups rather than backend-specific shortcuts.
