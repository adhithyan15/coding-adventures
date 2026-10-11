---
category: Testing & coverage
---

# A C compiler version probe does not prove the round-trip oracle can link

The PREP01 suffix-paste validation selected Swift's bundled `clang` because
its version command succeeded. Four existing C round-trip tests then failed
at native linking with exit 1181; the frontend and preprocessing tests passed.

Select an available, working compiler explicitly with the existing `SIR_CC`
override. Strawberry GCC completed all C library, rooted-input, and three-way
conformance tests in the same checkout. Report the setup failure and the
successful compiler used, rather than treating a version response or skipped
native tests as proof that the oracle ran. Do not change compiler behavior to
repair an unrelated local linker environment.
