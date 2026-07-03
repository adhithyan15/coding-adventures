# ADJ52 calibration-regression harness

The deterministic, offline gate that stops calibration fixes from secretly
adding entropy. Spec: [`ADJ54`](../../../ADJ54-calibration-regression-harness.md).

## Why

The run-2 softmax "calibration fix" made things worse and nobody could see it,
because it was judged on the noisy end-to-end blind-judge loop at n=3. The
engine is **deterministic**, so we freeze a corpus of `(rulebook, program,
label)` tuples once and score any engine change in milliseconds per case — no
LLM, no noise — with metrics that decompose **correctness** from
**calibration**. The gate fails on ANY per-case regression, even when the
aggregate improves.

## Files

| file | what |
|---|---|
| `score.py` | the scorer + gate (`score` and `diff` modes) |
| `corpus.json` | 30-case frozen golden corpus (artifacts in `../cases/case-N/`) |
| `rootcause.workflow.js` | the workflow that root-caused the wrong cases to levers H1/H2/H3 |
| `rootcause-results.json` | per-case lever assignments + evidence |
| `out/baseline.json` | pre-fix scores |
| `out/h2.json` | scores after the ADJ54 H2 fix |

## Usage

```bash
# score the current engine against the frozen corpus
mise exec -- python3 score.py score corpus.json out/after.json

# THE GATE: what regressed, per case? (fails on any correctness/confidence regression)
mise exec -- python3 score.py diff out/baseline.json out/after.json
```

## How it scores (the key design)

- **Ranking is on the RAW posterior** (`Posterior: P = …`) — preserves
  correctness; a calibration-only fix must never change who wins.
- **Calibration is on the REPORTED posterior** (`Reported (H2 …): P = …`) —
  the tempered value. On a pre-H2 engine that prints no Reported line,
  reported == raw.
- Differential candidates = every query except `next_step(…)` /
  recommendation queries.
- `correct_term` labels: engine top-1 for the 25 top-1-correct cases; the
  true-diagnosis term from root-cause for the 5 genuinely misranked cases.

## Adding a case

Append `{id, rulebook, program, correct_term}` to `corpus.json` (paths
relative to the crate manifest dir) and drop its `rulebook.adj` + `program.adj`
under `../cases/<id>/`. Re-baseline before measuring a fix against it.
