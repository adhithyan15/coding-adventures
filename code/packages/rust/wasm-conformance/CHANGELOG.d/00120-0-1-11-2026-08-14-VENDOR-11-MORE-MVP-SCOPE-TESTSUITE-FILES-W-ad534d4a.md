## 0.1.11 — 2026-08-14 — vendor 11 more MVP-scope testsuite files (WASM08)

Extends the vendored slice (same pinned commit, `28864811cf03bdbf88073378
6148feaba339582d` -- no re-pin) with 11 more WASM 1.0 MVP-core `.wast`
files, chosen by fetching the full upstream file listing at that pin and
filtering out anything referencing `"spectest"` or a bulk-memory/
reference-type table op (`table.get/set/size/grow/copy/fill/init`,
`memory.copy/fill/init`, `elem.drop`, `data.drop`) -- the same exclusion
criteria the original W05/PR3 slice used, just applied to the files that
slice didn't cover yet:

- `unreached-invalid.wast`, `unreached-valid.wast` -- dead-code type
  checking, directly exercising WASM06's new instruction-level type
  checker from a different angle than this repo's own hand-written
  tests. `unreached-invalid.wast`: 71/71 `assert_invalid` (100%).
  `unreached-valid.wast` currently fails to parse (`ref.is_null`, a
  reference-types proposal instruction outside this repo's scope) --
  vendored anyway since that's the same honestly-tracked "failed to
  parse" outcome several original-slice files already have for the same
  reference-types reason (`global.wast`, `select.wast`, `br_table.wast`,
  `call_indirect.wast`), not a new category of gap.
- `left-to-right.wast` (operand evaluation order): 95/95 `assert_return`.
- `memory_redundancy.wast`: 4/4 `assert_return` + 3/3 `action` (100%).
- `type.wast` (type-section declaration syntax): 1/1 `assert_malformed`.
- `obsolete-keywords.wast` (renamed-instruction rejection):
  11/11 `assert_malformed`.
- `float_memory.wast` currently fails to parse -- the same pre-existing
  "expected 1 or 2 limit numbers" gap `memory.wast`/`memory_grow.wast`/
  `float_exprs.wast` already have in the original slice.
- `id.wast` currently fails to parse -- an entire file dedicated to
  quoted-string identifier syntax (`$"arbitrary bytes"`), which this
  repo's hand-rolled tokenizer only supports for bare `$name` identifiers.
  A real, actionable gap, but out of this PR's "vendor more corpus" scope
  to also implement.
- `stack.wast` currently fails to parse -- `if $label ... else $label
  ... end $label` (an optional matching label repeated after `else`/
  `end`), a real WAT syntax gap in `wasm-wast-parser`'s `if` handling.
  Same "out of scope for this PR" reasoning as `id.wast`.
- `custom.wast` (custom-section handling): 6/8 `assert_malformed`.
- `utf8-invalid-encoding.wast`: all 176 cases `not_yet_supported` (UTF-8
  string-encoding validation in the binary format isn't implemented yet)
  -- correctly graded as such, not `Fail`.

Baseline: `assert_invalid` 838/838 -> 909/909 (+71, all from
`unreached-invalid.wast`). `assert_return` 13775/13777 -> 13874/13876
(+99). `assert_malformed` 51/53 -> 69/73 (+18 pass, +199
`not_yet_supported`, mostly `utf8-invalid-encoding.wast`'s 176 cases).
`module` 102/102 -> 108/108 (+6, one per newly-parsing file). Verified
via a full per-file diff against the pre-change baseline: zero
regressions on any pre-existing file.

