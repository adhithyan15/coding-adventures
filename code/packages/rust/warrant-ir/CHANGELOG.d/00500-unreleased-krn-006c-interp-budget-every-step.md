## Unreleased

- Fix KRN-006c (VER25 section 8.10): the reference interpreter checked its step budget only inside the instruction loop and in `call_fn`, so a cycle of blocks with no instructions (`Jump` back and forth) never checked and spun forever. The budget is now checked before every step, including terminators (top of each block visit, and before the terminator), so such a run ends with `BudgetExceeded(StepLimitExceeded)`. The cost schedule is unchanged (KRN-006a), including the existing double charge of ALU steps (VER25 section 8.9). `tests/test_interp_budget.rs` runs the interpreter on a worker thread with a 5 second timeout so a regression fails instead of hanging.
- Convert `CHANGELOG.md` to a sharded `CHANGELOG.d/` (one file per entry, history unchanged).
