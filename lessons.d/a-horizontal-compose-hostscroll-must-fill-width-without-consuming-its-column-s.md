---
category: Mosaic compiler pipeline
---

# A horizontal Compose HostScroll must fill width without consuming its Column's vertical budget

Compose's `Modifier.fillMaxSize()` bounds both axes. That is appropriate for a
vertical or two-axis page viewport, but a horizontal-only `HostScroll` inside a
`Column` then claims the parent's remaining height and measures later siblings
at zero height. TaskApp exposed the defect when its compact view switch became
horizontally scrollable and the following storage summary was present in the
semantics tree with zero-size bounds. Lower a horizontal-only viewport with
`fillMaxWidth()` plus `horizontalScroll(...)`; reserve `fillMaxSize()` for
vertical and two-axis viewports, then run the generated consumer's UI
conformance test rather than relying on emitter unit tests alone.
