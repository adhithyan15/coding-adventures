---
category: Testing & coverage
---

# A test that needs a level the runner lacks must pass levels explicitly, not borrow the one it happens to lack today

When the Python lane gained `ir.fixed` and `ir.tuple` (slice S10e), seven tests of the `ir.core` era
failed although nothing they tested had broken. Each wanted "a construct of a level the runner lacks"
and got it by writing a `bit_not`, a tuple-typed parameter or a case that `requires` `ir.fixed`, relying
on the lane's default `IMPLEMENTED_LEVELS` being `{ir.core}`. The default moved, so the construct was
live and the test's premise vanished.

The fix is to say which levels the runner has. `Program(..., levels=frozenset({"ir.core"}))` and
`run_kit(..., levels=...)` take the set as a parameter for exactly this reason, so a test about
`level-missing`, a not-live model or a skipped case pins the set it means. Where a test cannot pass
`levels` (a suite row built through `build_suite`), use a tag of a level that is still unimplemented
(`some`, an `ir.data` tag) rather than one of the level just added. A level that a later slice
implements will then move no test that is about *lacking* a level.
