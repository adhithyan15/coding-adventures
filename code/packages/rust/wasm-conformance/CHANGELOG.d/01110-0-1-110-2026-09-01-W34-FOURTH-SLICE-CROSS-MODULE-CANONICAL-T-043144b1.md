## 0.1.110 — 2026-09-01 — W34 fourth slice: cross-module canonical type-group equivalence (epic closed)

`CrossModuleFunction` (this crate's own `HostFunction` impl standing in
for "another WASM module's export" in its cross-module linking path) now
implements both of `HostFunction`'s new W34 fourth-slice methods, the
concrete piece this epic's whole "wire canonical equivalence into
CROSS-MODULE linking" design depended on (`code/specs/
W34-wasm-gc-canonical-type-equivalence.md`):

- **`canonical_type()`**: returns the EXPORTING instance's own already-
  computed `canonical_types[idx]` entry for this function's flat type-
  section index (`instance.canonical_types`, cloned once — cheap,
  `Rc`-backed — at `resolve_function` time, alongside the pre-existing
  `group_shape`/`is_final` computation).
- **`canonically_matches(target)`**: climbs the exporting instance's own
  `type_subtyping` chain (via the new `wasm_types::canonical_chain_
  reaches`), starting at this function's own type index (kept in a new
  `type_idx: Option<u32>` field, re-borrowing `instance` lazily rather
  than precomputing the whole chain eagerly) — the real cross-module
  subtyping rule `type-subtyping.wast`'s `M6`/`M7` "Linking" cases need
  (see `wasm-runtime`'s own CHANGELOG for how this was found: an earlier
  version of this slice used plain equivalence and regressed those two
  cases).
- 2 new end-to-end tests (`run_wast_source`, this crate's own top-level
  entry point, exactly how the real conformance harness exercises this
  path): `cross_module_isomorphic_rec_groups_with_no_shared_numbering_
  link_successfully` (the MVP.md/`type-equivalence.wast` "Isomorphic
  recursive types" headline shape — two mutually-referencing types, no
  shared numbering, importer's group padded to a different absolute
  index) and `cross_module_copy_paste_shaped_type_mismatch_is_correctly_
  rejected` (the `M5`-shaped negative case: superficially similar type
  names reused across modules, but one internal reference wired to a
  DIFFERENT earlier group than its counterpart — must NOT be accepted).

### Corpus impact

Baseline regenerated and diffed programmatically against the pre-slice
baseline across all 257 files — see `wasm-runtime`'s own CHANGELOG for
the full per-file, per-directive accounting (2 files changed, 5
directive-level improvements, zero regressions). This crate's own
`tests/fixtures/testsuite-status.json` is updated to match.

