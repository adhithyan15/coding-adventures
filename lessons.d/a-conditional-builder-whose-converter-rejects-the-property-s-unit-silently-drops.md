---
category: Mosaic compiler pipeline
---

# A conditional builder whose converter rejects the property's unit silently drops every override and emits a constant

`mosaic-emit-qt` has two scalar builders that turn a base value plus state
layers into a nested ternary. They differ in one thing: the converter they use
to read a layer's value.

- `conditional_number_expr` converts with `qml_number_or_none`, which **rejects
  any value carrying a unit**. It is for bare numbers like `opacity`.
- `conditional_px_expr` converts with `qml_px_or_none`, which strips `px`.

Routing `border-width` and `border-radius` through the number builder compiled,
ran, and produced a conditional expression made only of the base -- because
every `2px` override failed conversion and was filtered out. The output is a
constant that renders, so nothing crashes and nothing looks obviously wrong.

This passed the unit tests and the package gate. What caught it was an
assertion on the exact emitted string.

**What to do**

- When a builder takes a converter, the converter is part of the contract.
  Check it against the property's unit at every call site, not just the one
  you are adding.
- Assert the emitted string, not "the property is present". Only the string
  separates "the expression was built" from "the expression was built out of
  nothing".
- A filter that silently discards unconvertible input will express this bug
  class again. Where it matters, prefer a converter that makes a rejected
  override visible rather than absent.

Related: Qt assembles a `Rectangle`'s paint properties in FOUR separate places,
and a property wired into three of them is silently dropped by the fourth.
