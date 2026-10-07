---
category: Compiler / VM / language pipeline
---

# A BEAM closure result can be any but its return instruction needs a concrete type

While adding BEAM13's instruction-order regression, a minimal `call_closure`
module used `type_hint == "any"` for both the call and its `ret`. The validator
correctly rejected the return as an untyped instruction before the new root
assertion could run. Keep `call_closure`'s result hint as `any`, but give the
enclosing function and typed `ret` the concrete result type expected by the
test (for example `i64`). Run the focused test before interpreting a failed
assertion as evidence about the lowering under study.
