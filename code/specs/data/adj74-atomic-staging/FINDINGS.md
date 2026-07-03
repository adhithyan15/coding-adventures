# ADJ74 — atomic staging for small models (honest: rescues 0.5b, hurts 1.5b)

Does decomposing the byte-accounting contract into ONE instruction per turn (building
context incrementally) lift small models above the monolithic-prompt floor found in
ADJ73? Three arms (bare / monolithic / staged-5-turns), buried-override items, Ollama.

## Results — PS accuracy (override-correct) and skim-rate

| model | bare | mono | staged | skim bare | skim mono | skim staged |
|---|---:|---:|---:|---:|---:|---:|
| qwen2.5:0.5b | 0.25 | **0.00** | **0.58** | 0.75 | 0.67 | 0.42 |
| qwen2.5:1.5b | 0.50 | 0.42 | **0.17** | 0.50 | 0.50 | 0.75 |
| qwen2.5:3b | 0.83 | 0.67 | 0.67 | 0.08 | 0.33 | 0.33 |

AB abstention: 0.5b 0.25/0.25/0.12 ; 1.5b 0.62/0.88/0.88 ; 3b 1.00/0.62/0.62.

## Honest read

- **Staging rescued the 0.5b model: monolithic 0.00 → staged 0.58.** Atomizing the
  contract saved a model that *completely failed* the single giant instruction. This is a
  real, striking data point for the "decompose for small models" idea, and it cuts the
  0.5b skim-rate 0.75 → 0.42.
- **But it did not generalize.** Staging *hurt* the 1.5b (0.50 → 0.17): over the 5 turns
  the 1.5b drifted into meta-commentary about statements instead of committing an answer
  (the "other" class dominated its staged final answers). The 3b doesn't benefit (bare is
  best on these toy items; the contract adds overhead).
- AB is mixed: staging/mono helped 1.5b abstention (0.62 → 0.88) but hurt 0.5b and 3b.

## Conclusion
Atomic staging is a **model-specific rescue, not a general lift.** It can save a model
below the monolithic floor (0.5b) but can also induce drift in a slightly larger one
(1.5b). Combined with ADJ73, the honest picture is that the small-model behavioral
interventions on these synthetic items are noisy and model-specific. The clean wins
remain the *real* cases (Palmyrene, hummingbird) and the defensibility axis — not these
toy mechanistic ablations.
