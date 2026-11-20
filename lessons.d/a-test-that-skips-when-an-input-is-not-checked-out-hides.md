---
category: Testing & coverage
---

# A test that skips when an input is not checked out hides failures in a sparse worktree; count the skips and fetch those inputs before pushing

**What went wrong.** CV2-S11c-1 (D71) gave `missing-error` a template for an `Option`
payload, so logic-gates' `tri_state` row instantiates. The test
`test_every_bound_logic_gates_row_of_the_four_faults_instantiates_and_compiles_except_the_option_payload`
in `python/warrant-oracle/tests/test_scalar_faults.py` still asserted 21 rows and one refusal.
Its comment had been updated but its assertions had not. Locally it never ran: it calls
`pytest.skip` when `code/packages/rust/logic-gates/evidence.bindings` is absent, and the slice
was developed in a sparse worktree without that file, so the run reported "10 skipped" and
looked green. CI, with the full tree, failed `build (ubuntu-latest)` (`assert 22 == 21`).

**Fix.** Assert 22 instantiated rows, `tri_state` among them, and no refused row; drop
"except the option payload" from the test name.

**Next time.** In a sparse worktree, read the skip count, not only the pass count. Run
`pytest -rs` to list skip reasons, and for every skip that names a missing input file,
`git sparse-checkout add` that path and re-run before pushing. Changing what a fault,
template or level accepts means re-running every adopter test (logic-gates, arithmetic,
x25519, wasm-leb128), whether or not it is checked out.
