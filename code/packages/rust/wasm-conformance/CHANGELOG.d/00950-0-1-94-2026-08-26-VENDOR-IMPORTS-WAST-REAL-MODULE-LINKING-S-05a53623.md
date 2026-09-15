## 0.1.94 — 2026-08-26 — vendor imports.wast, real module linking's own corpus (task #61)

### Added

- Vendored the real, pinned-commit `imports.wast` (24002 bytes, 218
  directives) to `TESTSUITE_FILES` — closes out backlog item task #61
  ("blocked on tag/exceptions-proposal parsing"). W10 first attempted to
  vendor this file and found it failed to **parse** entirely: its
  "auxiliary modules to import from" preamble declares `(tag ...)`/
  `(tag (import ...) ...)`/`(export "x" (tag $y))`, WebAssembly-
  exceptions-proposal syntax `wasm-wast-parser` had zero grammar support
  for at the time (see 0.1.15's "Deferred, not silently dropped" entry
  below). The W21–W24 exceptions-proposal epic (real `tag` definitions,
  `tag` imports/exports, cross-instance tag identity) closed that gap as
  a side effect — re-fetching and re-running the file live, BEFORE any
  code change, confirmed it now parses in full and grades honestly.

### Fixed

- Real corpus, measured (`cargo run --bin wasm_conformance_report --
  --write-baseline`, full JSON diff against the pre-change baseline —
  confirmed every already-vendored file's tally is byte-for-byte
  unchanged): `imports.wast` grades `assert_unlinkable` 93/93 (100%,
  real `Pass`, including every `tag`-specific case — missing `tag`
  import, `tag` param-type mismatch, `func`-vs-`tag` kind mismatch),
  `module` 43 `Pass` + 25 `NotYetSupported`, `register` 6/6 (100%),
  `assert_invalid` +1 `Pass`. Zero new `Fail` anywhere in the file. The
  25 `NotYetSupported` `module` directives (plus 16 `assert_malformed`,
  26 `assert_return`, 8 `assert_trap`) are ALL pre-existing,
  already-documented capability gaps unrelated to `tag`: imports from
  the unimplemented `spectest` host module (explicitly out of scope,
  see `RegistryHost`'s own doc comment and W10's "Explicitly out of
  scope"), and `assert_malformed` cases needing "imports must precede
  all other definitions" ordering validation this crate's parser
  doesn't do (the same "text parsed without error" gap already present
  in a dozen other already-vendored files, e.g. `align.wast`/
  `block.wast`/`func.wast`). No `wasm-wast-parser`/`wasm-validator`/
  `wasm-runtime`/`wasm-execution` code change was needed — the real
  linking machinery (`RegistryHost`, `WasmRuntime::instantiate`'s
  link-failure path) built for W10, plus the tag infra built for
  W21–W24, already fully cover this file.

See `code/specs/W10-wasm-real-linking-and-unlinkable.md` (original
scope) and `code/specs/W21-wasm-exceptions-tag-throw-slice.md` through
`code/specs/W24-wasm-exceptions-exnref-catch-ref.md` (the tag infra that
unblocked it).

