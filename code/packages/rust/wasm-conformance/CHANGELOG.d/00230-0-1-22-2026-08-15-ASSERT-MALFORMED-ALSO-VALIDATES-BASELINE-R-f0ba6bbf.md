## 0.1.22 — 2026-08-15 — assert_malformed also validates; baseline regen (tasks #82-84)

### Fixed

- `grade_assert_malformed`'s binary path used to only call
  `WasmModuleParser::parse` -- a module that parses fine but is
  structurally malformed by a rule this crate's parser doesn't check at
  PARSE time (e.g. a memop's align flags with the reserved top bit set,
  which `wasm-module-parser` never decodes at all since code-section
  bytes are stored raw) went undetected even though `wasm-validator`'s
  instruction-level type-checker already rejects it (via its existing
  `align > max_align` check, just under a different error message than
  the spec's own "malformed memop flags" wording). Now also calls
  `self.runtime.validate(&built)` after a successful parse and grades
  `Pass` if THAT fails too -- same "outcome category, not the specific
  reason" precedent `grade_assert_unlinkable` already uses. Found via a
  prioritization scan after task #80 (PR #11844); fixes `align.wast`'s
  "memop flags" `assert_malformed` cases with zero new decode logic.
- Baseline regen following `wasm-module-parser` 0.2.4 (tasks #82/#84:
  malformed mutability bytes, data count section cross-check) and the
  `grade_assert_malformed` fix above: `align.wast` 0/2 -> 2/2,
  `custom.wast` 6/8 -> 8/8, `global.wast` 3/7 -> 7/7 (all
  `assert_malformed`), aggregate `assert_malformed` 208/216 -> 216/216
  (100%). Zero regressions anywhere else in the 61-file vendored corpus
  (verified via a full before/after diff of every file's per-kind tally).

