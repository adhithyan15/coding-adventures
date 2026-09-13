# UI80: Client-Side Image Map Semantics

## Status

Accepted for Venture's shared browser pipeline.

## Problem

The HTML parser already retained `img[usemap]`, `map`, and `area` metadata, but
the rendered image had no connection to those hidden nodes. Every host saw one
rectangular image and could not activate its authored regions. Implementing
maps in SwiftUI, WinUI, Qt, Flutter, or Compose would duplicate coordinate,
overlap, transform, and navigation policy.

## Shared Contract

`html-parser` carries the normalized `usemap` reference into the browser render
tree. `html-to-layout` resolves that local fragment to the first matching map
and attaches its navigable areas to the replaced image as typed Layout IR
metadata. Missing maps and non-local references remain inert.

The source coordinate space is the image's positive authored width and height
when present, otherwise its laid-out size. `html-to-paint` scales `rect`,
`circle`, `poly`, and `default` areas into the image box, composes the same
affine transform and ancestor clips used by paint, and emits ordinary shared
link regions. Invalid or degenerate coordinates do not produce a hit target.

When areas overlap, document order wins. Area target, download, opener, and
referrer metadata enters the existing shared link activation planner, so image
maps cannot bypass browsing-context mediation.

## Accessibility

Each rendered area has a stable key derived from its map and authored area ID,
falling back to document order. Its `alt` text is the accessible name, with the
resolved URL as a bounded fallback. `BrowserSession` exposes these projections
and activates a key through the same transaction used by pointer navigation.

## Host Boundary

Generated and native hosts continue to forward page coordinates or semantic
accessibility keys. They do not parse area coordinates, choose overlap order,
resolve targets, or mutate navigation directly. Existing pointer surfaces gain
image-map behavior without new toolkit-owned widgets.

## Acceptance

- Rectangles, scaled circles, polygons, and default regions hit only their
  authored geometry.
- Transforms, scrolling, fixed positioning, and overflow clips use the shared
  link coordinate pipeline.
- The first matching area wins when regions overlap.
- Pointer and accessibility activation preserve transactional current-context,
  auxiliary-context, and download behavior.
- Invalid, missing, or degenerate maps remain inert without host fallback.
