# UI81: Sequential Page Focus Navigation

## Status

Accepted for Venture's shared browser pipeline.

## Problem

Venture previously cycled Tab only through form controls. Links and details
summaries were pointer-accessible but absent from keyboard order, positive and
negative `tabindex` had no rendered effect, and most generated native hosts did
not forward Tab to the shared session. Letting each toolkit fill that gap would
produce different order, modal behavior, activation, and focus styling.

## Shared Contract

`html-to-layout` assigns stable document order and normalized non-negative
`tabindex` metadata to rendered links, enabled controls, and details summaries.
`html-to-paint` carries that metadata with transformed, clipped hit geometry.

`BrowserSession` orders positive `tabindex` values first by value and document
order, followed by zero and naturally focusable targets in document order.
Negative `tabindex`, disabled, hidden, inert, and accessibility-hidden targets
do not enter sequential traversal. An open modal restricts the candidate list
to interactive regions carrying its shared top-layer ancestry.

Tab and Shift+Tab wrap deterministically. Focusing a target clears incompatible
control or disclosure state, scrolls non-fixed geometry into view, and exposes
one reusable accessibility snapshot. Enter activates a focused link through
the existing transactional browsing-context planner; Enter or Space toggles a
focused details summary. Shared paint instructions draw non-control focus rings
while controls retain their existing focused presentation.

## Host Boundary

SwiftUI, WinUI, Qt, Flutter, Compose, and direct native adapters translate only
the semantic Tab key and Shift modifier. They do not enumerate elements, sort
`tabindex`, trap modal focus, scroll targets, choose activation behavior, or
draw focus state.

## Acceptance

- Positive `tabindex` targets precede natural order and preserve ties by
  document order.
- Negative, disabled, hidden, inert, and accessibility-hidden targets are
  skipped.
- Mixed link, control, and disclosure traversal wraps in both directions.
- Modal surfaces contain traversal and focused geometry is scrolled into view.
- Keyboard activation shares pointer navigation, top-layer, and form policy.
- Every native host forwards Tab and Shift without toolkit-owned focus order.
