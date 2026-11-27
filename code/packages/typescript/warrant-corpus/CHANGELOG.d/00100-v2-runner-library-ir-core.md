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

### Review fixes (CV2-S7a review, blockers B1 and B2 and findings S1 to S9, N1 to N10)

- **B1.** `slim_chunks` is validated wherever a suite row is read, on the corpus
  path too (at most 32, the first and last chunk, strictly ascending integers
  below `m`): `[]`, `7`, `[0]` and `[-1]` are refused and no longer pass with zero
  chunks or crash. `checkWork` is exported, called by `runSuite`, re-validates
  every slim chunk (no `NaN` total), counts negative-control re-runs in the slim
  total and compares with `!(x <= limit)`.
- **B2.** Every table keyed by a name read from input (`SCALARS`,
  `LEVEL_OF_KIND`, `LEAF_SIZES`) is a `Map`: `constructor`, `toString`, `valueOf`,
  `hasOwnProperty`, `__proto__` and `isPrototypeOf` are ordinary names (`type` in
  `v2-value`, a legal error type, `descriptor` for a descriptor kind).
- **S1.** A model body is compiled from a work list, not by re-entering `compile`
  through every callee: 64 chained models with 62-deep bodies at
  `max_ir_cost` 65,536 compile and evaluate.
- **S2.** `expect.type` keeps byte widths (`Bytes<2>`); a bare literal is
  `Bytes<len>`.
- **S3.** `v2-fault` honours the case's `levels` intersected with the runner's.
- **S4.** `loadSuite` scans every expression by tag name before it records
  `level-missing`: an unknown tag, a misplaced `subject_op`, a call out of
  `model_order` or a tree over 64 deep cannot hide in a not-live suite.
- **S5.** `v2-chunk` turns `level-missing` into `corpus` (R69 e).
- **S6.** Every component of a path below the fixture directory is checked with
  `lstat` and a symlink is refused, not followed (a symlinked `chunks/` included).
- **S7.** A duplicated control id and a duplicated fault row are refused.
- **S8.** `model_order` must name exactly the models (`model-order`, `corpus` in a
  corpus); every listed model of the lane's levels is typed, costed and chain-checked.
- **S9.** `oracle.subject` must fit `result_type`, and a `reference` suite's body
  must be `subject == expected` in either order (section 4.3).
- **Disagreement 11.** A chunk file consistent with its row but not with the
  enumeration is no longer `chunk-file` in the kit: the lane reports what it
  recomputes (section 11 `v2-chunk` (b), (c)). `calls` must still sum to
  `expected_calls` (section 5.4).
- **Nits.** N1 duplicate parameter and binder names refused. N2 an op's `requires`
  naming an undefined level is a corpus error. N3 a missing chunk file or `chunks/`
  directory is a clean `chunk-file` or `corpus` error; a fault that does not
  compile is `corpus`. N4 `omitted_examples: []` and a malformed list are refused,
  a suite with no admitted case is refused, `levels` must be the union of the
  `requires`. N5 an undetermined result type, a missing or mismatched stand-in and
  a wrong `args` count are harness errors. N6 a missing level is named with
  `firstMissing` everywhere (`List<Option<U8>>` names `ir.list`). N8 domain values
  are shared, with the reason written down. N9 a mutant is killed only by an
  outcome that differs, never by a harness crash. N10 the `ill-typed` messages are
  reworded.
- New `runCase` (one kit case) is exported for tests. 35 tests in
  `tests/v2/review-fixes.test.ts`; the kit is still 782 of 782, every mutant killed.
