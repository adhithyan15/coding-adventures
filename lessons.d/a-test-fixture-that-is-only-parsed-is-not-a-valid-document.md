---
category: Rust
---

# A test fixture that is only parsed is not a valid document, and breaks the day a rule type-checks it

`vos-link`'s `check-component` fixture crate (`tests/component.rs`) wrote a document whose claims called operations it
never declared (`f(v) == f(v)`), because every rule then in force (`declared_names`, `claim_calls`, CC09's
`bound_facts`) only PARSED it. VOS-039c's CC11 needs the shape of `model build`, which the Warrant analysis gets only
after `check_warrant` type-checks the document, and all eleven `-any` fixtures failed at once with a type error
(W0401, missing secrecy labels) that had nothing to do with the rule under test.

The fix was a second, valid document for kind `any`: a real `Impl(x, c)` with the oracle last, a `build`, the operations
the claims call, and the `labels` blocks the checker wants, plus bindings that agree with it (a `struct Impl` line and
`ret=Impl`). When a new rule moves a fixture from "parsed" to "analysed", build the fixture as the real thing, and
make each control assert the rule's own finding text so a type error cannot pass for it.
