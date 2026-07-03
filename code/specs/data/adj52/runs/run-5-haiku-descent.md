# ADJ52 run-5 — Haiku descent: framework *discipline* without the engine

Workflow `wjw8ir3gh`: **100/100 completed, 0 skipped, perturbation preserved the
diagnosis in 100/100.** 600 agents, 18.9M tokens, ~50 min. Both answering arms
are **Haiku**; Prepare (find + perturb + ground truth) and the blind judge stay
strong. Arm 1 = Haiku blind (plain answer). Arm 2 = Haiku + framework
*discipline* but **no engine/program**: Haiku decomposes an IR, derives a cited
rulebook from it, then reasons to a conclusion **citing the input facts and
rulebook rules** — no adj-lang, no posteriors. Full per-case data (prose, ground
truth, IR, rulebook, conclusion, citations, verdict) is in
`run-5-haiku-descent-100case-full.json`; trimmed table in
`run-5-haiku-descent-summary.json`.

## Scorecard

| metric | value |
|---|---|
| completed / attempted | 100 / 100 (0 skipped) |
| perturbation preserved dx | 100 / 100 |
| **framework-Haiku correct** | **38** |
| **blind-Haiku correct** | **36** |
| **blind-judge wins — framework / blind / tie** | **60 / 37 / 3** |

## Cross-tabs

- correctness: both 24, only-framework 14, only-blind 12, **neither 50**
- the 60 framework wins decompose as: **35 won-and-correct** + **25 won while
  *neither* arm was correct** (preferred on defensibility on a hard missed case)
- **framework won while blind was the actually-correct one: 0**
- **blind won while framework was the actually-correct one: 0**

That last pair is the clean property: **the judge's preference for the framework
arm never overrode correctness** — it only kicked in when the framework arm was
right, or when neither arm was right. Defensibility never beat a correct answer.

## The headline — contrast with the frontier rung

| | correctness | blind-judge wins |
|---|---|---|
| **Frontier Claude, WITH engine** (run-3) | 62 ≈ 61 (parity) | framework **39** / plain **60** — *lost* |
| **Haiku, framework as discipline, NO engine** (this run) | 38 ≈ 36 (slight edge) | framework **60** / blind **37** — *won* |

At the frontier the framework **lost** the blind comparison despite equal
correctness — dragged down by the engine's saturated posteriors and
pseudo-precise citations (run-3's finding: 28 losses were right-but-overconfident).
Here, with the **engine removed** and only the IR → cited-rulebook → grounded-
citation *discipline* in play, the framework **wins** the blind comparison
decisively — and at the *weaker* model. The discipline is a pure defensibility
gain once the false-precision numerics are gone.

## What this establishes

1. **The engine's false precision was the loss factor, not the structure.** Strip
   the engine and keep the decompose/derive/cite discipline, and the framework
   arm is preferred 60–37 — even at Haiku. The auditable, cited reasoning is what
   the judge rewards; the over-confident posteriors were what it punished.
2. **The descent thesis holds: the discipline helps the weak model.** It rescued
   14 cases blind-Haiku got wrong (net +2 on correctness), and produced a
   defensible, preferred answer on 25 hard cases *both* arms missed — the
   "diagnosable wrongness / defensible workup even on a miss" property. And it
   never traded a right answer for a pretty one (0/0).
3. **Reusable corpus.** Unlike run-3, every case's prose + ground truth + IR +
   rulebook + conclusion + citations + verdict is persisted — this 100 is the
   frozen corpus for the next rung down.

## Honest limits

- **neither-correct = 50/100.** Haiku misses half these hard masquerade cases
  regardless of arm (frontier's neither was 31/100 — Haiku is meaningfully weaker,
  as expected on the way down). The framework makes Haiku's output *more
  defensible* and *marginally more correct*, not a strong diagnostician on hard
  cases.
- **The correctness edge is small (+2).** The large effect is defensibility (60–37).
- **Two variables differ between the frontier and Haiku rows** (model rung AND
  engine-vs-discipline), so the "engine was the culprit" conclusion is strongly
  suggested but not airtight from these two runs alone.

## Next

The clean isolation: run **frontier Claude with the *same* no-engine discipline
arm** (decompose → cited rulebook → cited conclusion) — if it also wins the blind
comparison there, the engine-was-the-culprit conclusion is nailed. Then continue
the descent below Haiku.
