## 0.1.130 — 2026-09-02 — round-2 security review: bounded-trap proof for a `global.get`-alias read in a hot recursive loop (W35 fifth slice, round 2)

`wasm-runtime` 0.6.36 corrected two now-stale doc comments and fixed a
real, own-caught gap (`ValueType::NonNullConcreteFuncRef` missing from
`resolve_exported_global_funcrefs`'s own funcref-family type check) found
while building this release's own new test. See that crate's own
CHANGELOG for the full account.

New regression test,
`a_global_get_alias_of_an_imported_funcref_global_read_in_a_deep_tail_
recursion_loop_traps_cleanly_instead_of_corrupting_state`: `$B` imports
`$A`'s exported, `(ref $t)`-typed funcref global `g0` and declares a
NEVER-exported local alias `$galias` (`(global.get $g0)`) -- exactly the
new capability 0.6.35's unconditional (not export-scoped) propagation
introduced. `$B`'s own `count` function self-recurses via `return_call`,
reading `$galias` (and dropping the result) once per step -- structurally
the same "read a `func_ref: Some` global on every step of an unbounded
tail-recursive loop" shape `return_call_ref.wast`'s own `$count`/`$even`/
`$odd` already exercise (those stay safe because they're `ref.func`-
initialized, never `global.get`-initialized -- see `wasm-runtime`'s own
corrected doc comments). Confirmed: a count of 100 completes correctly;
a count one past `MAX_FUNC_REF_HEAP_LEN` traps cleanly (`assert_trap`,
message content unchecked per this harness's own established convention)
-- never a panic, hang, or silently wrong return value. Building this
test is what caught the `NonNullConcreteFuncRef` gap directly (the first
version of the test, using `(ref $t)` for `$A`'s exported global, failed
with a WRONG dispatch result before the type-check fix, not merely an
unexpected pass/fail on the trap assertion -- a real, reproduced bug this
test found on its own, not a hypothetical).

`cargo test -p wasm-conformance`: 74 (lib, +1) + 2 (integration) passed,
0 failed. Corpus baseline unaffected (re-diffed against the original
pre-fix baseline: still exactly the one `elem.wast` change, zero real
failures anywhere). `cargo clippy --release --all-targets`: clean.

