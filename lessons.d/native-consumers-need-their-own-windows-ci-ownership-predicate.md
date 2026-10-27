---
category: CI & GitHub Actions
---

# Native consumers need their own Windows CI ownership predicate

NN51's production planner with only cpp/neural-framework affected and
NEEDS_CPP=true selected Linux/macOS but no Windows. The actual Go diagnostic
plan also includes the parser prerequisite, which does select Windows, but
only its explicit parser step executes under the NN50 predicate: that is not
native training coverage. Add a narrow native-training
predicate, preserve all existing owners and the shared scheduler/bootstrap,
execute both cl and clang-cl command tests in the required BUILD roll-up, and
test direct/platform-override/unknown plans against the actual production code.
Review and refresh only the owned OCAML03 planner digest, retaining its pins,
OCaml predicates and mutation rejection tests.
