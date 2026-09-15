## 0.1.129 — 2026-09-02 — regression test for a security-review finding: `global.get`-copied funcref globals (W35 fifth slice, round 1)

`wasm-runtime` 0.6.35 fixed a real gap a pre-push security review found
in 0.1.128's own fix (immediately below): a LOCAL global initialized via
`(global.get $other)` -- copying another global's value rather than
minting its own via `ref.func` -- could have its `func_ref` resolved
against the WRONG instance's own function space, a silent wrong-function
dispatch. See that crate's own CHANGELOG for the full trace.

New hand-built end-to-end regression test,
`a_local_global_that_copies_an_imported_funcref_global_via_global_get_
propagates_the_real_source_identity_not_a_raw_index`: `$B` imports `$A`'s
exported funcref global `$g0`, defines its OWN local pass-through global
`$g1` (`(global.get $g0)`, also exported), and declares a decoy local
function (`$decoy`, returns 999) at the SAME numeric index `$A`'s real
function (`$ax`, returns 42) occupies in `$A`'s own space -- a table
populated via `$g1` and read through `call_indirect` must reach `$ax`
(42), not `$decoy` (999). Confirmed to fail (reaching `$decoy`) with the
fix reverted and pass with it applied -- a real A/B.

`cargo test -p wasm-conformance`: 73 (lib, +1) + 2 (integration) passed,
0 failed. Corpus baseline unaffected (re-diffed against the original
pre-0.1.128 baseline: still exactly the one `elem.wast` change, zero real
failures anywhere). `cargo clippy --release --all-targets`: clean.

