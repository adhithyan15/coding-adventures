## 0.1.10 — 2026-08-14 — baseline regenerated after the instruction-level type checker landed (WASM06)

`wasm-validator` 0.2.0 added a real per-instruction type checker (W02 Phase
2) to `validate()`. Baseline regenerated: `assert_invalid` 15/838 (826
`not_yet_supported`) → 838/838 (100%, only 3 `not_yet_supported`
remaining). `assert_return`/`module`/every other kind ended at the exact
same counts as before this change — zero regressions, verified via a full
per-file diff against the pre-change baseline.

Also fixed one stale test in this crate: `assert_invalid_accepted_by_structural_validator_is_not_yet_supported`
asserted the old "we can't tell" behavior for a module that's structurally
fine but semantically ill-typed (`(func (result i32))` with an empty
body). Now that `validate()` catches this for real, the case is correctly
graded `Pass`, not `NotYetSupported` — renamed and updated to assert that.

