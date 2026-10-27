---
category: Testing & coverage
---

# A hostile AST fixture must preserve required calls and compile before it proves shadow or post-helper overwrite rejection

An AST guard can appear to reject a dangerous rewrite for the wrong reason. A
fixture that deletes the required helper call is rejected by the elementary
"call is present" check, so it says nothing about whether an anchored direct
write, a nested same-name helper, or a whole-state overwrite would pass. A
fixture accepted only by `syn` can likewise contain names or types that would
never compile in the production context.

Keep the approved helper call in every bypass fixture, make the hostile source
a self-contained crate fragment, and run it through `rustc` before asserting
that the structural validator rejects it. For critical caller chains, exact-lock
the complete dispatcher AST as well as the leaf helpers, explicitly reject
nested local items with approved helper names, and scan each affected match arm
for `state.field = ...` or `*state = ...` after the helper call. This makes the
fixture prove the intended property instead of a weaker missing-anchor error.
