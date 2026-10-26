---
category: CI & GitHub Actions
---

# Shared CI planner edits must update the reviewed OCaml matrix digest without weakening its contract

NN47 head cb46896a0f3 passed its repaired Windows validation step, but actual
CI40244391795 contracts job121326114275 failed OCAML03: the shared matrix planner
digest remained 30ea6d06 after the scoped C Windows requirement changed its run
block to 056461d8. This is planner identity drift, not an OCaml toolchain or
parser failure. Review the exact planner delta, preserve NEEDS_OCAML/macOS and
Windows scheduling, pinned setup and mutation rejection checks, then refresh
only the reviewed digest. Run all affected contract tests before publication;
never remove the digest guard or accept arbitrary planner text to make CI green.
