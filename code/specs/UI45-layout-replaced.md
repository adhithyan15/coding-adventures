# UI45: Intrinsic Replaced Sizing

## Status

Implemented by the Rust `layout-replaced` crate and consumed by block, inline,
flex, grid, table, float, and paint composition.

## Contract

Decoded resources and other producers may attach finite positive dimensions:

```text
ext["replaced"] = {
  intrinsicWidth: number,
  intrinsicHeight: number,
  aspectRatio: number
}
```

The aspect ratio may override the resource ratio (for CSS `aspect-ratio`).
Malformed metadata is ignored and available through producer diagnostics.

## Used Size

Specified width and height win. If one axis is automatic, the preferred ratio
derives it. If both are automatic, intrinsic dimensions are used. Missing
dimensions fall back to the CSS replaced-element default of 300 by 150.

Minimum and maximum constraints clamp the used size. When an axis remains
automatic, ratio-preserving recomputation follows a clamp. Available inline
space is a final upper bound, so the same result feeds normal flow, inline
atoms, float shrink-to-fit, flex bases, grid intrinsic tracks, and table tracks.

## Resource Lifecycle

The browser commits pending images using default geometry. Once a retained
decoded pixel resource arrives, its width and height enter the HTML style/layout
context and the retained document reflows without refetching. Duplicate URLs
share one decoded intrinsic size.

## Object Fit

`fill` occupies the content box. `contain` uses the smaller scale, `cover` the
larger scale, and `none` preserves natural size. Non-fill results are centered.
Cover/none overflow emits a backend-neutral rectangular clip around the image
instruction; hosts receive ordinary paint commands and implement no sizing
policy.

## Acceptance

Coverage includes default and decoded dimensions, specified single axes,
preferred ratios, min/max clamps, float/flex/grid/table intrinsic widths,
contain/cover/fill/none rectangles, retained-resource reflow, and deterministic
browser fixture geometry and paint.

## Non-Goals

This phase does not add video/canvas intrinsic metadata, object-position,
responsive source selection, or EXIF orientation. Those features must extend
the reusable replaced contract.
