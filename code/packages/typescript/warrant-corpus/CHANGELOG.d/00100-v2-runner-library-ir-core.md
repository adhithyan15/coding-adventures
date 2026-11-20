## Unreleased - v2 runner library at `ir.core` (CV2-S7a)

- New `src/v2/` tree: the TypeScript lane's **second reading** of VER04r (with
  errata 16.12 to 16.20), VER04i and VER04j at level `ir.core`. It was written
  from the specification, the committed conformance kit and corpora, and this
  package's own v1 code, without reading the oracle, the generator or another
  lane's v2 code; the repository cannot prove that independence.
- Types (the section 2.1 grammar, depth 16, `Bytes<N>`, named static widths) and
  R51's `flows`/`unify`; values decoded by declared type (JSON, the canonical
  stream, TFV values); resolved descriptor trees (`size()`, `nth()`); the IR
  scanner, typer, worst-path cost model and closure compiler with the one
  wrapping rule [D4], the 2^127 window, the overflow class of R71, static widths
  for `subject_op` (R70), the bytes-`lit` type (R73), the `take`/`drop` count
  (R74), the 64-deep expression and model-chain limits (R77); suite rows with
  closed key sets and the R75 binder bounds, chunk files with every cross-check,
  chunk enumeration by unranking (chunk `c` alone equals the sequential walk),
  digests and caps; the three fault wrappers typed by a common type (R94).
- [D63] `list` descriptors, `List<T>` and the `list` tag are the level `ir.list`.
  [D64] `level-missing` is decided per suite, control, model and op by what is
  reached, in the three tiers of R69 (c). [D65] a not-live model or op gets no row.
- `runKit` runs the six `v2-*` kit categories by `requires`: run plus skipped
  equals the index count, `also_sequential` chunk cases are also walked from
  chunk 0, an unknown category (`v2-binding`) is digest-checked and skipped whole.
  All 782 cases pass, 0 failed, 0 skipped.
- `loadV2Corpus`/`runSuite` read the four committed v2 corpora at `ir.core`
  (ct-compare 9 of 9 suites, lzw 4, x25519 6, wasm-leb128 4; the rest
  `level-missing`; every `ir.core` fault row compiles) and reproduce every
  chunk's cases, calls and digests.
- 28 mutants (no wrap, eager `and`, first-binder-fastest, ...) each make the kit
  fail by case id (falsification first, [D42]).
- Source digest changes, so `typescript/ct-compare`'s v1 manifest is regenerated
  mechanically in the same change (its own changelog says so).
- Not in this change: the v2 corpus session, binding and manifest (S7b);
  `ct-compare` moving to v2 (S7c); `ir.fixed`, `ir.tuple` and `ir.data`.
