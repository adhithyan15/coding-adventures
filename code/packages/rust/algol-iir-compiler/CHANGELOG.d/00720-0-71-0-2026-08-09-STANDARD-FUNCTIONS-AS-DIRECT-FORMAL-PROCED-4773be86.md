## 0.71.0 — 2026-08-09 — standard functions as direct formal procedures

A direct `procedure` actual may now be any implemented ALGOL standard function:
`abs`, `sign`, `entier`, `sqrt`, `sin`, `cos`, `ln`, `exp`, or `arctan`. A
specialised wrapper retains the function target and reuses its existing inline
lowering at the formal call site, preserving normal arity/type validation and
avoiding a function-pointer or closure ABI. Dynamic procedure values remain
unsupported.

