# How low can the decomposer go?

The warm path asks the local model for exactly one thing: decompose messy clinical
prose into typed findings in the closed dictionary. Everything downstream is the
same 0-answer-time-model-call CPU engine over the same grounded rulebook. So the
model is a swappable part — and `bench_models.py` measures how **small** it can be
before the diagnoses degrade, on the 4 meningitis vignettes.

```sh
python3 bench/bench_models.py            # strict normalizer
python3 bench/bench_models.py --tolerant # framework absorbs small-model JSON variance
```

## Results (correct / 4 cases; 0 wrong in every single cell)

| model         | size   | STRICT framework | TOLERANT framework |
|---------------|--------|:----------------:|:------------------:|
| gemma4:latest | 9.6 GB | 2/4              | 2/4                |
| llama3.1:8b   | 4.9 GB | **4/4**          | **4/4**            |
| qwen2.5:3b    | 1.9 GB | 2/4              | 3/4                |
| qwen2.5:1.5b  | 986 MB | 0/4              | **4/4**            |
| qwen2.5:0.5b  | 397 MB | 0/4              | 0/4 (1/4 parse)    |

## What it shows

1. **The floor is set by the framework, not the model.** With a strict normalizer
   the floor is ~8B. Teaching the deterministic normalizer to absorb the JSON
   shapes small models emit (findings as a `{functor: value}` map, the functor in
   the `span` field, bare-string findings, stray strings in
   `inference_justifications`) drops the full-4/4 floor to **qwen2.5:1.5b — 986 MB,
   a 5x smaller model.** This is "intelligence in the framework, not the weights":
   the normalizer does more work so the model can do less.

2. **Every model fails SAFE — 0 wrong diagnoses at any size.** A model never
   produces *wrong* findings that yield a *wrong* diagnosis; it either decomposes
   correctly or fails to produce parseable findings (which abstains/fails). The
   closed-vocabulary gate drops any hallucinated functor/value before the engine
   sees it, so the catastrophic failure mode — a confident wrong answer — does not
   occur even at 0.5B.

3. **Bigger is not better.** gemma4 (9.6 GB, the *largest* model) scores 2/4 — it
   tends to emit prose / bare-string findings that don't map. The axis that
   matters is **structured-output discipline**, not parameter count.

4. **The hard floor is ~0.5B.** qwen2.5:0.5b (397 MB) parses valid JSON for only
   1/4 cases — below this the model cannot reliably emit structured output at all,
   which no amount of framework tolerance can fix. That is the genuine capability
   floor for this task.

## Honest limits
- 4 vignettes, one differential — directional, not a powered benchmark.
- Small models are noisy even at temperature 0 (qwen2.5:3b lands 2–3/4 across
  runs); treat the cells as approximate, the *trend* as the finding.
- The tolerant normalizer here is the bench's `tolerant_findings`. Promoting it
  into the production `warm/ir_to_adj.py` + `warm/decompose.py` would make the
  shipped warm pipeline support sub-2B decomposers directly — a clean follow-up.
- "Correct" = the engine's leader matches gold; it does not score decomposition
  completeness (a model can get the diagnosis right while missing a finding the
  rulebook didn't need).

## The takeaway
For this structured-extraction task, the practical floor with a tolerant framework
is **~1–1.5B parameters (≈1 GB)**, not 8B — and crucially, **no model size
misdiagnoses**, because correctness is enforced downstream by the grounded
rulebook and the closed-vocabulary gate, not by the model. Make the framework
absorb more, and the model can be smaller.
