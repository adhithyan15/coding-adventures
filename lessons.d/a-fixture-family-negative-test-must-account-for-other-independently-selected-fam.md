---
category: CI & GitHub Actions
---

# A fixture-family negative test must account for other independently selected families

The toolchain-detection CI selector's negative-path test treated a valid
`ci-gate-selection-*.json` fixture as if it must select zero packages. The
independent CI-gate selector correctly selected its four native readers, so
the new test failed even though the toolchain selector did not overmatch.
For a sibling family that has its own detector, assert that the new family's
*additional* roots are absent, or leave that sibling to its own positive test.
Reserve zero-selection assertions for paths outside every registered family.
