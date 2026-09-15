## 0.1.128 — 2026-09-02 — regenerated baseline: cross-instance funcref-GLOBAL fix -- W35 fifth slice (1 file, LAST real failure in the corpus closed)

Regenerated `tests/fixtures/testsuite-status.json` (`--write-baseline`)
after `wasm-runtime` 0.6.34 added `resolve_exported_global_funcrefs` (see
that crate's own CHANGELOG for the full root-cause trace and fix). Diffed
programmatically (Python, comparing the `files` dict) against the
pre-fix baseline (0.1.127) across all 257 files: exactly ONE file's
tally changed, zero elsewhere --

- `elem.wast`: `assert_return` 26/27 (1 real fail) → 27/27 (0 fail) --
  the "Initializing a table with imported funcref global" case, honestly
  diagnosed but deliberately left unfixed by the PR that landed W38
  slices 4/5 (0.1.127, above) as a genuine, out-of-scope-for-that-PR
  W35-level gap. This was the ONLY real (non-`not_yet_supported`) `Fail`
  anywhere in the entire 257-file corpus as of 0.1.127 -- confirming a
  corpus-wide result of ZERO real failures, for the first time in this
  campaign.

New hand-built end-to-end regression test,
`a_table_entry_populated_via_an_imported_funcref_global_dispatches_to_
the_exporters_own_function` -- a minimal reproduction of `elem.wast`'s
own failing case via `run_wast_source` directly (a funcref-typed global
populated via `ref.func` on a local function, exported, imported by a
second module, written into that module's own table through an active
elem segment's `(global.get 0)` item, and read back via `call_indirect`),
proving the fix end-to-end without depending on the vendored corpus file.

`cargo test -p wasm-conformance`: 72 (lib, +1) + 2 (integration) passed,
0 failed -- confirmed via a `git stash` A/B against the pre-fix tree (71
lib tests passed before, all still passing after, plus the 1 new one).
`cargo clippy --release --all-targets`: clean.

