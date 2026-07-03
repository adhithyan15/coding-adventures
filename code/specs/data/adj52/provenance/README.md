# provenance/ — byte-provenance method (how the corpus is built)

The tooling that enforces the [ADJ55](../../../ADJ55-provenance-first-corpus.md)
invariant: *every magnitude must point at a datum.* The built corpus (the product)
lives in [`../corpus/`](../corpus/).

## Two directions of the same spider

- **Backward audit** — `spider.workflow.js`: take an existing (invented) rulebook and,
  per claim, crawl to the cited source and grade the magnitude `grounded` /
  `direction_only` / `fabricated`. Run on case-5 urology → **0/19 grounded**, decisive
  clause `fabricated` (`spider-results.json`). This is how we *detect* hallucinated
  numbers.
- **Forward construction** — `pe/ground.workflow.js`: build the corpus from data. Per
  `finding → diagnosis` link, crawl to a primary study, byte-anchor the sens/spec/OR,
  compute the LR, admit only if grounded. Run on PE → **12/12 grounded**. This is how we
  *avoid* hallucinated numbers in the first place.

## Files

| file | role |
|---|---|
| `spider.workflow.js` | backward provenance audit of a rulebook's magnitudes |
| `spider-results.json` | case-5 audit output (0/19 grounded) |
| `build_tree.py` / `eval_tree.py` | case-5 tree-JSON rulebook + direct evaluator (`as_derived` / `grounded_only` / `direction_preserving` modes) |
| `case5-tree.json` | case-5 as a provenance-annotated tree |
| `pe/ground.workflow.js` | forward grounding spider (corpus construction) |
| `pe/findings.json` | the case-blind PE finding skeleton |
| `pe/grounding-results.json` | the 12 byte-anchored grounding chains |
| `pe/build_corpus.py` | assembles `../corpus/pulmonary_embolism/corpus.json` |
| `pe/eval_case.py` | deterministic case adjudication against the grounded corpus |
| `pe/case-*.json`, `pe/case-results.json` | the PMC11999957 validation case |

## The two results, together

`grounded_only` evaluation of case-5 dissolves its confident-wrong 0.99 to a base-rate
differential (the fabricated LRs stop pushing); forward-grounded PE adjudicates a real
Wells-0 PE correctly at 0.28 → 0.89 where an inventing deriver excluded it at 0.01.
Detection and prevention of hallucination, same invariant.
