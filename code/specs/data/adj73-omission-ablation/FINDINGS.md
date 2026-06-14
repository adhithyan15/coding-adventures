# ADJ73 — omission ablation: findings (honest, mixed, with two methodology lessons)

Three conditions (bare / coverage-only / justified-discards) × stratified items
(PS = present-but-skimmed override; AB = absent/uncovered) × 5 open-weights models via
Ollama, temp 0. 300 generations. Scored with a style-robust matcher (see "Lesson 2").

## Re-scored results

**PS accuracy (override-correct) | AB accuracy (abstained):**

| model | PS bare | PS cov | PS just | AB bare | AB cov | AB just |
|---|---:|---:|---:|---:|---:|---:|
| qwen2.5:1.5b | 0.50 | 0.25 | 0.25 | 0.88 | 0.50 | 0.38 |
| **qwen2.5:3b** | **0.58** | **0.50** | **0.83** | 1.00 | 0.75 | 0.75 |
| gemma4 | 1.00 | 1.00 | 0.92 | 0.88 | 0.88 | 1.00 |
| llama3.1:8b | 0.83 | 0.67 | 0.83 | 0.62 | 0.12 | 0.12 |
| qwen2.5:14b | 1.00 | 1.00 | 1.00 | 0.75 | 1.00 | 0.88 |

**PS skim-trap rate (lower = better):**

| model | bare | coverage | justified |
|---|---:|---:|---:|
| qwen2.5:1.5b | 0.50 | 0.58 | 0.67 |
| **qwen2.5:3b** | **0.33** | **0.42** | **0.17** |
| gemma4 | 0.00 | 0.00 | 0.00 |
| llama3.1:8b | 0.17 | 0.33 | 0.17 |
| qwen2.5:14b | 0.00 | 0.00 | 0.00 |

## What the data honestly shows

1. **Where omission actually occurs (qwen2.5:3b), justified-discards is the lever.**
   Bare skims 33% of PS items (accuracy 0.58). Justified-discards cuts skimming to 17%
   and lifts accuracy to 0.83. **Coverage-only does NOT help** (skim 0.42, accuracy 0.50)
   — you can tag the override `[DISCARD]` without engaging it. This is the clean
   isolating result the experiment was built to test: the *justification requirement*,
   not the bookkeeping, attacks omission.

2. **The benefit is conditional on the failure being present.** Capable models (gemma4,
   qwen2.5:14b) do not skim these items at all (bare PS ≈ 1.00, skim 0.00), so the
   framework is neutral — it cannot fix a failure that isn't happening. This is on-thesis:
   the contract is a *targeted intervention for omission*, not a general accuracy booster.
   It also means these synthetic items are **too easy to induce omission in capable
   models** — a real limitation (see "Next").

3. **There is a capability floor (qwen2.5:1.5b).** The 1.5b model cannot reliably follow
   the clause-listing + justification instruction; the added structure *distracts* it and
   the framework hurts (PS 0.50 → 0.25). Reproduces ADJ67's floor.

4. **Honest trade-off — justified can increase fabrication on ABSENT items for weaker
   models.** AB fabricate-rate rose under justified for qwen2.5:1.5b (0.12 → 0.62) and
   llama3.1:8b (0.25 → 0.62): forcing an answer-with-justification can push a weak model
   to invent rather than abstain on uncovered questions. Capable models went the right way
   (gemma4 0.12 → 0.00). **This motivates the discrimination/abstention gate (ADJ67) as a
   necessary companion to the contract — the contract alone is not enough on weak models.**

## Two methodology lessons (both caught, both relevant to the paper)

- **Lesson 1 — item difficulty must be calibrated to induce the failure.** v1 items put the
  override as a salient "however..." second sentence; bare accuracy was ~0.92 (no omission
  to fix). v2 buries the override after elaborating the general rule, which induces
  skimming at the 3b scale. Caught by the smoke test.
- **Lesson 2 — scoring must be style-invariant.** The justified condition produces
  natural-language final answers ("No sales tax applies", "$0.00", "0 days"); a brittle
  token matcher scored these as wrong, *biasing the experiment against the framework* and
  falsely showing "framework hurts capable models." Re-scoring from saved raw outputs with
  a style-robust matcher removed the artifact. (Lesson for the paper: when conditions
  change output style, use style-invariant scoring.)

## Bottom line (what to claim)

The clean, defensible claim: **where omission occurs, forced justification of discards —
not mere coverage — reduces it.** The honest scope: the effect is conditional on the
failure being present, has a capability floor, and can trade toward fabrication on absent
items for weak models (motivating the abstention gate). This is a careful mechanistic
*pilot*, not a slam-dunk, and it is reported as such.

## Next (to make this paper-grade)
- **Adversarial items that induce omission in *capable* models** (the Palmyrene regime,
  where even Opus skimmed). The synthetic rule-override items top out at the 3b scale; the
  mechanism's test on frontier models needs items that actually trip them.
- Larger n per cell (currently 12 PS / 8 AB) with confidence intervals.
- Pair with the **abstention/discrimination gate** and re-measure the AB fabrication
  trade-off.
