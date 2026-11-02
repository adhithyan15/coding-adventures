---
category: Testing & coverage
---

# A sparse worktree must materialise every path an explicit repository contract reads

The FM-B075 spec-map test was invoked after its program and live-site inputs
were added to a sparse worktree, but one of its explicit package README inputs
was still absent. The test failed with `FileNotFoundError` before it could
evaluate the roadmap assertions being validated.

Before running a repository contract in a sparse worktree, read the contract's
declared path inventory and materialise every input, including documentation
files outside the edited package. A missing sparse path is not a product
failure, but a partial run is also not validation evidence.

NN51's shared CI planner audit likewise needed the OCaml package/program
inventory and Go build-tool executor source for OCAML03, despite editing only
a C++ Windows predicate. The first local contract run failed on those absent
sparse inputs. Adding their exact paths restored all 55 tests (two Windows
symlink skips) and the repository validator; no OCaml pins/checks were weakened.
