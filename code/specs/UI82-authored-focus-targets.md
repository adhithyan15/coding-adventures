# UI82: Authored Focus Targets

## Status

Accepted for Venture's shared browser pipeline.

## Problem

UI81 unified links, controls, and details summaries, but two rendered focus
classes remained outside that model: client-side image-map areas and ordinary
elements made focusable by `tabindex` or `contenteditable`. Leaving those gaps
to native toolkits would make keyboard order, modal containment, scrolling,
focus presentation, and accessibility roles differ by host.

## Shared Contract

`html-to-layout` allocates image-map area focus slots with the associated
rendered image and projects authored generic focusability, accessible names,
roles, editing mode, normalized non-negative `tabindex`, and document order.
Negative, disabled, hidden, inert, and accessibility-hidden targets remain
outside sequential traversal.

`html-to-paint` turns generic targets into transformed, clipped `FocusRegion`
values and carries focus metadata on shape-aware image-map link regions. The
geometry keeps fixed-position and top-layer ancestry so focus behavior follows
the same visual coordinate system as paint and pointer hit testing.

`BrowserSession` merges those regions with links, controls, and disclosures,
sorts positive `tabindex` before ordinary order, traps all classes inside the
topmost modal, scrolls non-fixed targets into view, publishes reusable role,
name, state, and geometry snapshots, and paints the shared focus ring. Enter on
a focused image-map area reuses the existing transactional link and browsing-
context planner. Generic authored targets are focusable but do not invent
script activation or editing behavior that Venture does not yet implement.

## Host Boundary

All hosts continue to forward only semantic Tab and Shift+Tab. They do not
enumerate image-map areas, inspect `tabindex` or `contenteditable`, infer roles,
sort targets, trap modal focus, scroll geometry, or draw focus state. No new
generated-host ABI is required.

## Acceptance

- Positive and ordinary image-map areas participate in the shared order.
- Authored generic `tabindex` and contenteditable targets expose stable keys,
  accessible names, and roles; negative targets are excluded.
- Transforms, clips, fixed positioning, and modal ancestry survive projection.
- Tab and Shift+Tab wrap through every target class with one focus owner.
- Enter on an image-map area shares ordinary link target/download policy.
- Existing native host Tab forwarding inherits the behavior without new host
  policy or generated source branches.
