---
category: Mosaic compiler pipeline
---

# Optional profiling component references must follow layout root rewrites

Adding component arguments to native profiling introduced fixed-root references that the layout-variant rewrite did not replace. Five emitter tests caught the invariant failure. Rewrite both profiling calls to the active layout root and run the full emitter suite, not only the plain-window path. Local C# template string assertions also require LF line endings; preserve them when editing embedded templates on Windows.
