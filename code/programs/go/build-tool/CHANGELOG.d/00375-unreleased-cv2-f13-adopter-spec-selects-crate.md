## Unreleased - an adopter's .warrant spec edit selects its own Rust crate (CV2-F13)

- A spec-only edit to `code/specs/<crate>.warrant` now selects `rust/<crate>`
  for every non-VOS Rust crate whose BUILD runs `gen_evidence -- --check
  <crate>`: arithmetic, ct-compare, gf256, hash-functions, logic-gates, lzw,
  wasm-leb128, x25519 and zeroize (the VOS crates already did, through
  `vosImplementationRow`). Previously the four corpus-v2 adopters (ct-compare,
  x25519, lzw, wasm-leb128) selected only the generator, the oracle and
  `rust/warrant-grammar`, and arithmetic, gf256, hash-functions, logic-gates
  and zeroize selected only `rust/warrant-grammar`, so the evidence check ran
  first in an unrelated later PR.
- Nine inline `warrantEvidenceRow` entries in `repositoryInputConsumers` (exact
  path match, consumer added with prerequisites, not dependents, like the other
  repository-input rows); no `init()` and no separate crate list.
- New row `go/programs/build-tool`, selected by exactly
  `code/packages/rust/<dir>/evidence.bindings` (one segment, glob
  `code/packages/rust/*/evidence.bindings`), so the PR that adds an adopter runs
  the coverage test below. BUILD edits are deliberately not a trigger: any Rust
  BUILD edit would select the build tool.
- Tests: `TestWarrantSpecEditSelectsTheCrateThatChecksEvidenceAgainstIt` (every
  spec selects its crate and exactly the consumers it selected before; lookalike
  negatives; fail-closed on a missing crate, naming it; paths pinned to the
  predicate), `TestWarrantEvidenceRowsCoverEveryEvidenceCrate` (every directory
  with `evidence.bindings` or a `gen_evidence ... --check` BUILD line, in any
  argument order, has a `rust/<dir>` row matching its spec, VOS crates included;
  the reverse check fails a stale row; the scanned file count is logged) and
  `TestRustEvidenceBindingsSelectTheBuildTool`. Three existing expectations
  gained the crate they now select.
