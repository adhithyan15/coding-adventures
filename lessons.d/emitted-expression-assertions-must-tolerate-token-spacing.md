---
category: Testing & coverage
---

# Emitted expression assertions must tolerate token spacing

The Board drop-target test looked for `col[1]`, but the compiler emits the
equivalent expression `(( col [ 1 ] ))`. The test failed to locate the control
before checking its geometry. Inspect the emitted expression first, normalize
only insignificant whitespace when identifying it, and keep the actual geometry
assertion exact. Do not weaken a missing-control failure into a global substring
check that could match an unrelated node.
