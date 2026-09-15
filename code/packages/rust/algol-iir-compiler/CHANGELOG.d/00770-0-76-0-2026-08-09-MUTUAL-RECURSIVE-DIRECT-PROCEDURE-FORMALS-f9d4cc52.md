## 0.76.0 — 2026-08-09 — mutual recursive direct procedure formals

Regression coverage now proves that two mutually recursive direct `procedure`
formals reuse their active specialised sibling pair. The direct target survives
the finite recursion graph without a function-pointer, closure, or descriptor
value in the IIR ABI.

