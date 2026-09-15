## 0.1.100 — 2026-08-31 — data.wast/annotations.wast/inline-module.wast vendored (extended-const, annotations, inline-module)

Vendors `data.wast`, `annotations.wast`, `inline-module.wast` from the
pinned SHA (`28864811cf03bdbf880733786148feaba339582d`) — three files a
prior batch-vendoring session investigated and deliberately skipped,
each with a real, independent, previously-undiagnosed gap. Each was
fetched fresh and read in full for this pass, not assumed from the prior
session's guesses — one of the three (`inline-module.wast`) needed
something genuinely different from what had been guessed.

- **`data.wast`**: needed the extended-const proposal
  (`i32.add`/`i32.sub`/`i32.mul` inside a data segment's offset
  expression, and nested combinations of those with `global.get`). Fixed
  in `wasm-execution` (`evaluate_const_expr`'s single-value accumulator
  replaced with a real operand stack) — see that crate's own CHANGELOG
  and `code/specs/W29-wasm-extended-const.md`. `wasm-wast-parser`'s text
  encoder and `wasm-module-parser`'s binary reader needed no changes.
  Real numbers: `module` 14/14 (100%, +17 not-yet-supported — the file's
  own `spectest`-memory-importing modules, a pre-existing, unrelated
  gap), `assert_invalid` 6/6 (100%, +14 not-yet-supported — const-expr
  TYPE errors this crate's validator has never checked, pre-existing,
  unrelated to extended-const), `assert_unlinkable` 14/14 (100%).
- **`annotations.wast`**: needed real support for WAT's custom `(@id
  ...)` out-of-band annotation syntax. Fixed in `wasm-wast-parser` (a
  whole-tree annotation-stripping pass, plus a dedicated permissive
  tokenizer scan for an annotation's body — see that crate's own
  CHANGELOG for the full breakdown). Real numbers: `module` 4/4 (100%,
  +6 not-yet-supported — `spectest` imports, same pre-existing gap as
  above), `assert_malformed` 62/64 (97%, +2 not-yet-supported).
- **`inline-module.wast`**: a prior session's lightweight scan guessed
  this needed the same `(module definition ...)`/`(module instance
  ...)` generative-instantiation form `instance.wast` needs (still out
  of scope, see below) — the real fetched file is something simpler and
  different: a whole `.wast` script with NO `(module ...)` wrapper at
  all, just bare module fields (`(func) (memory 0) (func (export
  "f"))`) at the top level. Fixed in `wasm-wast-parser::script::
  parse_script` (extends the "abbreviated module" shorthand
  `module.rs::parse_module` already supported for `.wat` text/`module
  quote` bodies to a whole script). `module` 1/1 (100%) — a real,
  unconditional pass.

Full before/after baseline diff confirms zero tally change in any of the
233 previously-vendored files — purely additive (`module` +19 pass
across the three new files, `assert_invalid` +6, `assert_malformed` +62,
`assert_unlinkable` +14, matching each file's own numbers above exactly).

Still deliberately out of scope, unchanged from the prior session's
assessment: `instance.wast` (the `(module definition ...)`/`(module
instance ...)` generative-instantiation form — genuinely distinct from
`inline-module.wast`'s gap, confirmed by actually reading both real
files this pass) and the `binary.wast`/`binary-leb128.wast`/
`binary_leb128_64.wast` family (already vendored in the prior 0.1.99
release, unaffected here).

