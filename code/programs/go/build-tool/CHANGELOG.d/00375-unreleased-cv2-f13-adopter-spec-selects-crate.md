## Unreleased - an adopter's .warrant spec edit selects its own Rust crate (CV2-F13)

- A spec-only edit to `code/specs/<crate>.warrant` now selects `rust/<crate>`
  for every non-VOS Rust crate whose BUILD runs `gen_evidence -- --check
  <crate>`: arithmetic, ct-compare, gf256, hash-functions, logic-gates, lzw,
  wasm-leb128, x25519 and zeroize (the VOS crates already did, through
  `vosImplementationRow`). Previously only the generator, the oracle and
  `rust/warrant-grammar` were selected, so the evidence check ran first in
  an unrelated later PR.
- New `warrantEvidenceCrates` table and `warrantEvidenceRow` (exact path match,
  consumer added with prerequisites, not dependents, like the other
  repository-input rows).
- Tests: `TestWarrantSpecEditSelectsTheCrateThatChecksEvidenceAgainstIt`
  (every spec selects its crate and the consumers it selected before; lookalike
  negatives; fail-closed on a missing crate; paths pinned to the predicate) and
  `TestWarrantEvidenceCratesCoverEveryGenEvidenceBuild` (the table equals the
  BUILD files). Two existing expectations gained the crate they now select.
