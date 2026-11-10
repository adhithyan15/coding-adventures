---
category: CI & GitHub Actions
---

# Editing the ci.yml "Plan build matrix" step needs a matching GENERIC_MATRIX_PLAN_RUN_SHA256 bump in ocaml_toolchain_lock.py

`code/scripts/ocaml_toolchain_lock.py` pins a SHA-256 of the `Plan build matrix`
step's `run:` text (`GENERIC_MATRIX_PLAN_RUN_SHA256`) so the OCaml predicates in
the generic CI planner cannot change unreviewed. Any edit to that script -- even
a comment or docstring -- changes the digest and fails
`test_ocaml_toolchain_lock.py` (`generic CI matrix planner must equal reviewed
digest ...`), which the `Repo-wide metadata contracts` job runs.

It surfaced while adding the daily cross-OS full build: the first targeted test
pass happened before the planner was edited, so it was green, and the failure
only showed on a re-run of every test that reads `ci.yml`.

Do not compute the new digest from your own YAML parse of the step: the checker
extracts the text its own way and the two differ. Run the test, copy the `found`
value from the error, and update the constant with a comment saying what the
planner change adds and that the OCaml parts are unchanged. Re-run *all* tests
that read `ci.yml` after the last edit to the workflow, not just the ones you
expect to be affected, and bump the digest again after any further planner edit.
