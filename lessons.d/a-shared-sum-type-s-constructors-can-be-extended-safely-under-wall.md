---
category: Haskell
---

# A shared sum type's constructors can be extended safely under `-Wall` without `-Werror`

— adding 5 new `PaintInstruction` constructors (`PaintGlyphRun`, `PaintLine`, `PaintGroup`, `PaintClip`, `PaintLayer`) to `paint-instructions` left every existing non-exhaustive `case`/pattern-match elsewhere in the repo (barcode/qr-code packages) compiling with only a missing-pattern *warning*, not an error. Still, grep every consumer for direct `PaintRect{...}`-style record construction (not just pattern matches) before extending a shared type — a producer that builds records positionally or partially would break, even though pattern-match consumers wouldn't.
