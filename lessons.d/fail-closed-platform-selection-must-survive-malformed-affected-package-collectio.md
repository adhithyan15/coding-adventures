---
category: CI & GitHub Actions
---

# Fail-closed platform selection must survive malformed affected-package collections

NN48's executable production-planner regression supplied a non-list
affected_packages value. The Windows requirement correctly selected its lane,
but needs_macos subsequently called set() on the noniterable value and aborted
the entire planner. Treat non-list macOS/Linux package collections like unknown
plans and retain the macOS runner. Preserve every known-plan and independent
OCaml/Rust Windows predicate. Since the shared run block is identity-guarded,
review this actual delta, recompute its parsed digest and rerun the OCAML03
mutation-rejection contracts; never bypass the digest to accept the repair.
