## 0.67.0 — 2026-08-09 — recursive array call-by-name (AL7)

Procedures whose name formals are arrays can now re-enter the direct specialised
sibling currently being lowered. Array actuals already use the ordinary typed
descriptor ABI, so direct and mutual recursion preserve the caller's backing
storage, bounds, and strides without a scalar thunk. Regression coverage fills
the same array through direct and mutually recursive procedures; scalar name
recursion remains explicitly rejected pending a runtime thunk ABI.

