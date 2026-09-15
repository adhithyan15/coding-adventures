## 0.1.112 — 2026-09-01 — W-next: RegistryHost's resolve_global now shares, doesn't copy

`RegistryHost::resolve_global` (this crate's own `HostInterface` impl,
resolving a `register`ed sibling module's export for real cross-module
linking, WASM05/W10) used to return `*instance.globals.get(index)?` — a
genuine VALUE COPY of whatever the exporting instance's global held AT
THAT EXACT MOMENT. Fixed to `instance.globals.get(index)?.clone()`,
cloning the `Rc<RefCell<WasmValue>>` POINTER instead (see `wasm-
execution`'s own CHANGELOG for the field-level fix this depends on) —
the exact same shape `resolve_memory`/`resolve_table`'s own `.cloned()`
already has (W28), just applied to the one remaining piece of instance
state that hadn't gotten it yet. `Action::Get`'s own global-read path
(`(get $M "name")` directives) now dereferences the shared cell
(`*g.borrow()`) rather than expecting an owned `WasmValue` back.

Root-caused via a fresh prioritization pass over the module-linking
corner of the conformance corpus (`instance.wast`/`linking.wast`'s own
`mut_glob`/"Import is not generative" tests) — see `wasm-runtime`'s own
CHANGELOG for the full bug-hunt writeup, including a second, unrelated
bug (active element segments applied after data segments instead of
before) found and fixed in the same pass, and a third, pre-existing gap
(table entries have no real cross-instance function identity) confirmed
as the sole remaining cause of everything else in this cluster.

### Corpus impact

See `wasm-runtime`'s own CHANGELOG for the full, programmatically-diffed
baseline accounting across all 257 files (this crate's own baseline file,
`tests/fixtures/testsuite-status.json`, is the artifact that diff reads).


