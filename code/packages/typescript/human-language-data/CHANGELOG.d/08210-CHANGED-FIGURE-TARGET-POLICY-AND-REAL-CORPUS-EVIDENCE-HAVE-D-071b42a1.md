### Changed — figure-target policy and real-corpus evidence have direct writing-system owners

- Moved enabled-track mappings, written-sign order, fused pairs and runs,
  apart-cluster evidence, shared-headline rules, font facts, and citations into
  direct TypeScript owners under `src/figure-target-policies/`.
- Reduced `src/figure-targets.ts` to a compatibility facade over a stable,
  script-independent core and a validated fail-closed composition boundary.
- Replaced the shared real-corpus figure-target case with writing-system-owned
  cases plus separate closure and printed-book placement evidence.
- Preserved the exact pre-migration real-corpus candidate set and every public
  compatibility export.
