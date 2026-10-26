---
category: CI & GitHub Actions
---

# Cpp toolchain selection does not guarantee a Windows build entry

NN50 executed the actual CI matrix planner with the dependency-aware affected
package `cpp/bounded-json` and `NEEDS_CPP=true`. The resulting matrix contained
only Ubuntu. Toolchain detection/bootstrap is not platform scheduling proof.

Before publication, add a narrow dependency-aware NN50 Windows requirement to
the existing planner/runner/bootstrap/required build roll-up and execute tests
against the production planner. Preserve unrelated C++ matrix policy. An owned
planner change also requires reviewing its OCAML03 identity contract; do not
refresh the digest without the actual owned change and mutation tests.
