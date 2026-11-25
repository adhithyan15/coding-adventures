## 0.1.0

- Initial Swift lane of VER05 (spec VER05k): `SplitMix64` / `fnv1a64` / `subSeed`
  (wrapping operators, `(0 &- n) % n` rejection threshold), a capped `BigInt`
  (1000 bits at the API, 1100 internal), the generator algebra (`Gens`), integrated
  shrinkers (`Shrink`), the hierarchical shrink-budget tree (`ShrinkBudget`), the
  honest `Runner` with CI redaction and the closed failure vocabulary, total and
  bounded `Canon` rendering, a hand-written strict `StrictJson` and `Tfv` validator,
  and the `Corpus` evaluator.
- Replays all 25 pbt-v1 cases from the committed fixture with exact outputs, minimals
  and step counts; negative controls corrupt every case, truncate the corpus and
  sabotage the engine through `@testable import` hooks.
