## 0.1.104 — 2026-08-31 — W32 second slice: call_ref/return_call_ref real numbers

Implements `code/specs/W32-wasm-non-null-concrete-reference-types.md`'s
second slice: `NonNullStructRef`/`NonNullConcreteFuncRef` (`wasm-types`
0.1.14), their subtyping rules (`wasm-validator` 0.2.75), `(ref $t)` text
parsing + `call_ref`/`return_call_ref` + declarative element segments
(`wasm-wast-parser` 0.1.87), and the runtime support needed to make all of
that soundly executable (`wasm-execution` 0.9.78). Corpus stays
**256/257** vendored files (no new file added this slice), but four
already-vendored files' real pass numbers moved substantially:

- `call_ref.wast`: `module` 0/4 → **4/4 (100%)**; `assert_return` 0/23 →
  **23/23 (100%)**; `assert_trap` 0/4 → **4/4 (100%)**; `assert_invalid`
  4/4 (100%, but PREVIOUSLY passing only via a lucky module-parse failure,
  not a real check) → 2/2 (100%) + 2 honestly `not_yet_supported` (now
  passing/failing for the RIGHT reasons — the 2 remaining gaps are
  `try_table`-adjacent dead-code/unreachable-stack edge cases, unrelated
  to this slice's own scope).
- `return_call_ref.wast`: `module` 0/5 → **4/4 (100%)** + 1
  `not_yet_supported`; `assert_return` 0/31 → **31/31 (100%)**;
  `assert_trap` 0/4 → **4/4 (100%)**; `assert_invalid` 11/11 (100%, same
  "lucky parse failure" caveat as above) → 9/9 (100%) + 2
  `not_yet_supported`.
- `struct.wast`/`array.wast`: `module` 1/6 → **0/6** and 1/7 → **0/7**
  respectively — a DECREASE, but a deliberate, honest correction, not a
  regression: both files' one "passing" module previously validated only
  because a pre-existing `wasm-wast-parser` bug silently misparsed a
  `(type ... (struct/array ...))` declaration into a bogus EMPTY `(func)`
  type (see `wasm-wast-parser` 0.1.87's own changelog entry) — this
  slice's `(ref $t)` parsing is what first made it possible to
  REFERENCE that bogus type in a way that validated, surfacing the
  latent bug via a full-corpus baseline diff. Fixed at the source; both
  files' real struct/array-type-declaration support remains a separate,
  larger, later slice (this crate's own `wasm-wast-parser` has no
  struct/array-type TEXT-format declarations at all).

Diffed the regenerated baseline programmatically against the pre-change
one across all 256 files: every file with a DECREASED pass count (9 of
them: `array.wast`, `call_ref.wast`, `func.wast`, `ref.wast`,
`return_call_ref.wast`, `struct.wast`, `try_table.wast`, `type-rec.wast`,
`unreached-invalid.wast`) is a deliberate, individually-investigated
reclassification — either a fixed "fake pass" (the struct/array bug
above) or a genuinely honest `not_yet_supported` for a confirmed,
separately-scoped gap (per-local definite-initialization tracking,
real recursive type groups' forward-reference rules, `try_table`'s own
catch-clause payload-type checking) — never a newly-introduced silent
misbehavior. 26 directive-kind counts improved (`elem.wast`,
`instance.wast`, `ref_func.wast`, `select.wast`, `table.wast`,
`table_grow.wast`, `type-equivalence.wast`, `unreached-valid.wast`, plus
the four headline files above) as a side effect of `ref.func`'s corrected
result type and the new `(ref $t)`/non-null-lattice/bounds-check plumbing.

