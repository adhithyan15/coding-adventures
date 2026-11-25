---
category: Compiler / VM / language pipeline
---

# An undeclared type name must be refused as a typing error before its level is read, or it reads as a missing level and hides the error

**What went wrong.** The TypeScript v2 runner (VER04r, S7a) decides `level-missing` by *reach*: a record, a variant and an `Option` are the level `ir.data`, so a lane at `ir.core` records a unit that reaches one as not-live. The first version read a type's level from its *syntax*, and a bare name parses as a record or variant. A model parameter of type `Nope`, which nothing declares, therefore read as "needs `ir.data`", the model was skipped as not-live, and the typing error never surfaced: a corrupt corpus became a quietly shorter one. Two tests caught it (`an undeclared parameter type` and a suite whose `result_type` was `Nope`).

**The fix.** Check that every name is *declared* (`checkNames`: an error type where a `Result` wants one, a record or variant elsewhere) **before** reading a type's level, in every place a level is decided (a model's parameters, an op's signature, a binder, a result type, an environment). A name that is declared and of a missing level is not-live; one that is not declared is a typing error of any level.

**What to do differently.** When a refusal is *scoped by reach* (a missing level, a feature flag, a capability), check what is *wrong* before what is *merely absent*: a scoped refusal must never be able to hide an unconditional one. Give the unconditional checks (names, key sets, tag names, depth) their own pass that runs over the unreached part too.
