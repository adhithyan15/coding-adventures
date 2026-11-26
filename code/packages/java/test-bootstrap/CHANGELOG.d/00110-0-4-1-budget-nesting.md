## 0.4.1 - nested-check budget fixes (VER05e section 4.1)

- **Security fix.** `Runner.check` runs its whole body (case loop included) in a scope (root,
  or child of an ambient scope) capped at numTests + shrink attempts. A nested check charges
  one attempt at entry (refused: property-exhausted, nothing evaluated), one per later case and
  the size of each sampled collection. Nested checks in the generation phase and branching
  properties are now bounded by the outermost root. Replaces the "first case is free" rule.
- Re-verification charges the check scope; a refusal reports the original witness with 0 steps.
- `shrinkSequence` with maxSteps 0 returns the input uncopied; `Gen.shrink`/`shrinkNested`
  validate maxSteps; `filter` charges before its user predicate runs.
- Documented: lazily created pool workers may inherit a concurrent check's ambient scope;
  package-private statics rely on package privacy (seal the jar).
