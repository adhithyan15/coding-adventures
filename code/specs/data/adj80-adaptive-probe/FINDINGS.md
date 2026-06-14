# ADJ80 — adaptive capability probe: naive probes FAIL; empirical bake-off is the answer

The idea (Adhithya): the framework runs probe prompts, figures out what the model can do,
and adapts its protocol — instead of one monolithic approach for all models.

## Naive probe (generic format/JSON/multi-objective) — does NOT predict capability

| model | P1 format | P2 json | P3 multi-obj | probe tier | reality (ADJ74/77) |
|---|---|---|---|---|---|
| qwen2.5:0.5b | ok | ok | ok | **T2 capable** | **WRONG** — collapses on the real contract (0.00) |
| qwen2.5:1.5b | FAIL | ok | FAIL | **T0 sub-floor** | **WRONG** — handles bare/mono (~0.5) |
| qwen2.5:3b | ok | ok | FAIL | T1 mid | ~ok (mid) |
| gemma4 / llama3.1:8b / qwen2.5:14b | ok | ok | ok | T2 | ok (capable) |

**The generic probes mis-tier the small models.** A 0.5B can emit `APPLE/7/DONE` and valid
JSON yet fail the multi-objective *content* contract; a 1.5B can flake on a toy format yet
do the real task. The probes test **surface format-following**; the real failure (ADJ76) is
**multi-objective cognitive load on content**. Surface proxies don't predict it.

## The fix (the idea, corrected): empirical protocol bake-off

Do not *predict* capability from proxies. **Run the candidate protocols (monolithic /
staged-natural / atomic-natural) on a small labeled CALIBRATION set and adopt whichever wins
for that model.** It tests the actual thing.

The existing ADJ74/ADJ77 data shows a bake-off would select correctly:
- qwen2.5:0.5b -> ATOMIC (monolithic 0.00; atomic/staged ~0.5-0.58)
- qwen2.5:1.5b -> BARE/MONOLITHIC (free-staging *hurt* it: 0.17)
- qwen2.5:3b -> SCAFFOLD (1.00 vs bare 0.83)
- gemma4 / llama3.1:8b / qwen2.5:14b -> MONOLITHIC (bare ~0.83-1.00)

So the adaptive layer is: **calibration bake-off -> per-model protocol selection**, not
capability-prediction from toy probes. Robust, and validated by prior data.

## Bottom line
The adaptive-framework instinct is right and important — but the selector must be an
**empirical bake-off on a mini-version of the real task**, because cheap generic probes are
not predictive of multi-objective content capability. Reported as a negative result on the
naive probe + the corrected design. Next: implement the automated calibration bake-off
(run 3 protocols x 3 calibration items per model, pick the winner) and wire it as the
framework's front door.
