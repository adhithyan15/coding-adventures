# UI44: Fragmented Inline Box Edges

## Status

Implemented by the Rust `layout-inline-box` crate and consumed by
`layout-inline`.

## Purpose

Semantic inline containers may cross line boundaries. Their margin, padding,
border, background, hit geometry, and continuation policy must fragment with
the inline content instead of becoming one oversized rectangle or disappearing.
This contract keeps that policy independent from HTML and host toolkits.

## IR Contract

A producer keeps physical margins and padding on the `LayoutNode`, border
widths in the existing paint extension, and may add:

```text
ext["inlineBox"] = { decorationBreak: "slice" | "clone" }
```

Missing or malformed metadata resolves to `slice`; producer diagnostics expose
unsupported values without making layout fallible.

## Fragmentation

`slice` treats all line fragments as one continuous box. Inline-start margin,
padding, and border appear only on the first fragment, and inline-end edges
appear only on the last. Block-start and block-end padding and borders apply to
every occupied line.

`clone` repeats both inline edges on every fragment. The formatter reserves
those edges before fitting text so decoration cannot overlap content or exceed
the selected line region unexpectedly.

Nested wrappers transition by their common prefix. Closing edges are reserved
inside-out and opening edges outside-in. Explicit breaks, Unicode wrapping,
float-driven line regions, and end-of-run closure use the same transition.

## Geometry, Paint, and Hit Testing

Each semantic wrapper is rebuilt once per occupied line. Its positioned box is
expanded from tight descendant bounds by the resolved fragment padding and
border; margins remain outside that border box. Descendants shift into the
content box, suppressed slice borders are removed from paint metadata, and
fragment identity is retained in `ext["inlineFragment"]`.

Because ordinary positioned boxes carry the final geometry, background/border
paint and link hit regions need no inline-specific backend or host behavior.

## Acceptance

Coverage requires slice and clone continuation, nested transition widths,
vertical line expansion, computed CSS mapping, wrapped semantic links, recursive
paint, and one link hit rectangle per visual fragment through the shared Venture
fixture pipeline.

## Non-Goals

This phase does not implement bidirectional logical-edge remapping,
multi-column/page fragmentation, border-radius corner slicing, or replaced
element intrinsic sizing. Those features must extend the shared contracts.
