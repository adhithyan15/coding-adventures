## 0.75.0 — 2026-08-09 — recursive direct procedure formals

Regression coverage now proves that a recursive direct `procedure` formal
reuses its active specialised sibling at each recursive call. Its static target
survives to the base case without a function-pointer, closure, or descriptor
value in the IIR ABI.

