# UI43: Reusable Float and Exclusion Formatting

## Status

Implemented by the Rust `layout-float` crate and consumed by `layout-block`.

## Purpose

CSS floats are not a paint offset. They remove a box from normal block
stacking while leaving a vertical exclusion that narrows subsequent line and
block opportunities. This specification keeps that state in a reusable layout
component so HTML producers, native hosts, and paint backends do not implement
their own float rules.

## IR Contract

A `LayoutNode` may carry:

```text
ext["float"] = {
  side:  "none" | "left" | "right",
  clear: "none" | "left" | "right" | "both"
}
```

Unknown or malformed values resolve to `none` and are available through a
stable producer diagnostic. Missing metadata is equivalent to both defaults.

## Formatting Context

Each block formatting context owns an `ExclusionSpace` whose inline size is
the containing block's content width. An exclusion records side, x/y origin,
width, and height. Width and height include the float's margins; the positioned
child remains at its border-box origin inside that footprint.

At a candidate vertical coordinate, active left exclusions move the available
band's left edge rightward and active right exclusions move its right edge
leftward. Opposing and same-side floats may share a band when their combined
footprints fit. Otherwise placement advances to the nearest active exclusion
bottom and retries. Geometry is finite, non-negative, and clamped to the
formatting-context width.

Inline formatting asks for a fresh region per line. A line that does not fit
wraps without moving already placed atoms; the next line re-evaluates active
exclusions and widens automatically after their bottom edges. Semantic wrapper
fragmentation and baseline alignment therefore remain owned by `layout-inline`.

## Sizing

An auto-sized float uses CSS shrink-to-fit:

```text
used = min(max(preferred-minimum, available), preferred)
```

The available width excludes active floats and the candidate's horizontal
margins. Text preferred width is its unwrapped measure; preferred minimum is
the widest unbreakable segment. Container intrinsic widths recursively combine
inline children by sum and block children by maximum. Explicit, percentage,
minimum, and maximum widths retain their normal containing-block semantics.

## Clear

Before laying out any floating or in-flow box, `clear` advances its candidate
coordinate to the greatest bottom edge of matching floats:

- `left` considers left floats.
- `right` considers right floats.
- `both` considers both sides.
- `none` preserves the normal-flow coordinate.

Clearance is computed in the same context as placement, so nested formatting
contexts do not leak exclusions across component boundaries.

## Flow, Paint, and Hit Testing

Floats remain children of their containing positioned node in source order.
Their resolved geometry therefore follows the ordinary recursive
`layout-to-paint` and hit-testing paths. No float-specific paint command or
host API exists. The containing context includes the deepest float bottom in
its content extent, preventing accidental clipping in the shared Venture page
pipeline.

## Diagnostics and Degenerate Input

- Unsupported side/clear strings produce a diagnostic and use `none`.
- Non-finite and negative geometry clamps to zero.
- A float wider than its context clamps to the context width.
- Zero-height exclusions never create an infinite placement loop.
- Placement retries only at known exclusion bottoms, making termination
  deterministic.

## Acceptance

Required coverage includes:

1. Opposing and same-side floats sharing and exhausting bands.
2. Side-specific and `both` clearance.
3. Band changes as exclusions expire vertically.
4. Intrinsic minimum/preferred shrink-to-fit bounds.
5. Margins in exclusion footprints and float-containing heights.
6. Computed CSS mapping through HTML into shared positioned geometry.
7. Recursive paint and hit geometry through the browser fixture used by all
   available native and generated hosts.

## Non-Goals

This phase does not add shapes, multi-column fragmentation, or toolkit-native
float widgets. Those features must extend the exclusion contract rather than
bypass it.
