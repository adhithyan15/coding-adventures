## 0.1.0

- Initial OCaml lane of VER05 (VER05q): SplitMix64 on `int64`, FNV-1a, `bounded` with the
  rejection threshold computed unsigned, `bounded_big`, a hand-written `Bigint`, the generator
  algebra with explicit per-generator `canon`, integrated shrinking (integer walk with floored
  bisection, sequence shrinker), the hierarchical shrink-budget tree (VER05e 4.1) with an ambient
  `Domain.DLS` carrier and an explicit `?parent` override, the honest runner with CI redaction,
  strict JSON and TFV v1 parsers, and the pbt-v1 conformance evaluator: 25/25 with exact
  outputs, minimal witnesses and step counts.
- 61 Alcotest tests: the corpus with negative controls and engine sabotage, the pinned values the
  corpus cannot catch (rejection draw at seed 7046029254386353131, the 2^63+1 boundary,
  `bounded(1)` consuming a word, floored bisection from -81, evaluation order), a threshold of 0
  still passing the whole corpus, and one group per OCaml hazard (63-bit `int`, exceptions and
  `Sys.Break`/`Out_of_memory`, effects, domains, cyclic and functional user values, hostile JSON
  nesting).
- Measured coverage 97.5% of instrumented points (`bisect-ppx-report`); the residue is engine
  invariants of the corpus evaluator that only a broken engine can reach.
- Found while testing: `Prng.bounded_int` first guarded with `1 lsl 62`, which is `min_int` on a
  63-bit `int` and so accepted nothing meaningful; the guard is now `n >= 1`.
