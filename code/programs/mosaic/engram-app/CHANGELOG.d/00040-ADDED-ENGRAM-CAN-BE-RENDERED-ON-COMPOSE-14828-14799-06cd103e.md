### Added — Engram can be rendered on Compose (#14828, #14799)

Engram is gated on SwiftUI and Qt in CI and has no Compose lane, so nothing
produced a picture of it on that backend. `conformance/compose/EngramScreenshots.kt`
and `scripts/render-compose.sh` now do, in one command.

The first render it produced is not flattering, and both defects were then
measured against the semantics tree rather than read off the image:

- **UI60 (#14828), confirmed.** All eleven deck-stat and collection chips draw
  the count on top of its label — `[0]` and `[Total]` both at
  `Rect.fromLTRB(49.0, 247.0, ...)`, identical origins. Compose gives `Box` the
  overlay behaviour UI29 assigns to `Stack`.
- **#14837, new.** The composition root is `1280 x 728` inside a `1280 x 900`
  window: 172px of the app is unpainted. Same class of defect as Trestle's
  in #14798.

Every semantics gate stayed green through both, because a node drawn on top of
another node is still present, still named, and still "displayed". The harness
makes the same narrow claim Trestle's does — that rendering *succeeds* and
produces a surface of the size asked for — and is explicitly not a pixel-diff
gate.

The script asserts `nativeComplete` before rendering, so a picture of a
degraded fallback cannot be mistaken for a picture of the product.


