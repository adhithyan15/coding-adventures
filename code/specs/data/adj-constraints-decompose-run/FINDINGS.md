# FINDINGS — live decompose→solve over the adj-lang constraint sublanguage (D2)

**Model:** `llama3.1:8b` (local, via Ollama, temperature 0, seed 7).
**Cases:** 4 messy-prose constraint word problems, one per solver path.
**Recorded run:** `results/` (committed). Reproduce with `python3 run.py`.

## Headline

| metric | value |
|---|---|
| engine-solved to gold | **3 / 4** |
| **answer-time model calls** | **0** |
| decompose calls (the only model touchpoint) | 4 (one per case) |
| fabricated wrong answers | **0** |

The architecture holds: the model is called **only to transcribe** prose into
adj-lang; **every answer is the engine's**, produced by `adj-lang-cli` at zero
answer-time model calls. On the cases the model transcribed correctly, the engine
returned the exact optimum / value / verdict, each cited to the constraint that
determined it.

## Per case

| id | path | model's decomposition (committed `.adj`) | engine | gold | ✓ |
|---|---|---|---|---|---|
| `relief_allocation` | maximize | `maximize 4·hot_meals + 3·emergency_shelter` under budget + meals cap | **44** | 44 | ✅ |
| `production_cost` | minimize | `minimize 5·a + 8·b` under contract minimums | **980** | 980 | ✅ |
| `schedule_feasibility` | check | introduced a `build_start` intermediate, chained `build_start = design_finish`, `build_finish − build_start ≥ 20` | **unsat** | unsat | ✅ |
| `workshop_break_even` | solve | wrote `2000 + 15·attendees ≤ 65·attendees` (an inequality) | **unsupported** | 40 | ❌ |

The decompositions are clean and human-legible — e.g. for the schedule the model
*invented an intermediate variable* (`build_start`) to chain the precedence, and
the engine then proved the contradiction (design ≥ 28 forces build ≥ 48 > 45).

## The miss is the point

`workshop_break_even` is the **honest-failure** case. "Break even" is an equality
(`revenue = cost`); the model wrote the inequality `cost ≤ revenue` ("at least
cover cost"). Solving *for a unique value* under an inequality is ill-posed, so the
engine returned:

```json
{ "outcome": "unsupported", "reason": "inequality constraints — feasibility/LP is track C1/C2" }
```

It did **not** invent a number. This is the whole thesis in one case: a small
model's mis-transcription is **localized and legible** (you can read the one wrong
operator in the committed `.adj`), and the engine **never launders a bad
decomposition into a confident answer**. Contrast a monolithic LLM, which would
have emitted *some* number with no way to see where it went wrong.

## Why this matters (and what it is not)

- **Intelligence in the framework, not the weights.** An 8B local model — far
  below a frontier model — is enough, because all it does is map surface forms to
  a constrained grammar. The reasoning (exact rational LP / feasibility / linear
  solve) is the CPU engine's. This is the [[project_dumber_models_constrained_envs]]
  hypothesis made concrete for constraints.
- **Correctable, not just correct.** Every step is inspectable: the prose, the
  model's `.adj`, the engine's cited decision. The break-even miss is fixable by
  editing one operator in the `.adj` — no model retrain, no prompt surgery.
- **Not a benchmark.** One model, one prompt, four cases — a *demonstration of the
  architecture*, not a powered model-quality study. The number that matters is
  **answer-time model calls = 0**, not 3/4. A different/larger local model is a
  one-arg swap (`python3 run.py qwen2.5:3b`); the engine and the golden tests are
  unchanged.

## Reproducibility

- The **model** half is non-deterministic and Ollama is not in CI, so it is not
  asserted in CI. The committed `results/*.adj` capture exactly what `llama3.1:8b`
  produced on the recorded run.
- The **engine** half is deterministic and *is* asserted in CI:
  [`adj-lang-cli/tests/decompose_run.rs`](../../../packages/rust/adj-lang-cli/tests/decompose_run.rs)
  re-solves every committed `.adj` (44 / 980 / unsat / unsupported) with no model
  in the loop — proving the engine is a pure function of the decomposition and the
  model's non-determinism is fully quarantined.
