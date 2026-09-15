## 0.1.12 — 2026-08-15 — Ref comparison + regen baseline for funcref/externref (WASM17)

Adds `WasmValue::Ref` handling to `const_value_to_wasm_value`/
`value_matches_expected` (exact `ref.extern n`/`ref.null` literals, plus
the bare `(ref.null)`/`(ref.func)` wildcard forms that only ever appear as
an `assert_return` expectation) and regenerates the baseline against
`wasm-wast-parser` 0.1.7's new reference-types grammar.

No vendored-file-list change (same pinned commit). Aggregate tallies are
UNCHANGED (`module` 108/108, `assert_return` 13874/13876, etc.) -- verified
via a full structured diff against the pre-change baseline: every
already-parsing file's pass/fail counts are byte-for-byte identical, zero
regressions.

Three previously-`failed_to_parse` entries moved to a DIFFERENT, deeper
failure point (real progress, not yet a full pass -- each blocked on its
own separate, already-scoped-out gap):
- `global.wast`: `at byte 840: ... found "externref"` -> `at byte 17077:
  ... found "$g0"` (a named global inline-import shorthand gap, unrelated
  to reference types -- logged as its own backlog item, WASM19).
- `br_table.wast`: `at byte 50812: ... found "externref"` -> `at byte
  51401: ... found "list"` (a concrete `(ref null $t)` heap type --
  deliberately out of scope, see `code/specs/
  W08-wasm-funcref-externref.md`).
- `unreached-valid.wast`: `unknown instruction "ref.is_null"` -> `unknown
  instruction "call_ref"` (a tail-calls/GC-proposal instruction, out of
  scope).

Two entries UNCHANGED, each blocked immediately by its own separate,
already-scoped-out gap (not touched by this PR):
- `select.wast`: `unknown instruction "result"` -- `select (result T)` is
  a SEPARATE opcode (0x1C) from plain `select` (0x1B), needed to
  disambiguate a reference-typed `select`'s result; genuinely out of this
  PR's scope (see `wasm-wast-parser`'s own
  `select_with_explicit_result_type_annotation_is_a_known_gap` test).
- `call_indirect.wast`: `expected an index, found "func"` -- the extended
  active-`elem`-segment syntax (`(elem (table $t) (i32.const 0) func $g
  $h)`), a bulk-memory-adjacent grammar this PR's spec already excludes.

