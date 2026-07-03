# The local-model pivot — privacy / HIPAA by architecture

MYCIN-2026's warm path is **one** small-local-model call (decompose messy prose →
typed findings) followed by a **CPU engine** that does the diagnosis, the
value-of-information, and the treatment set-cover with **0 answer-time model
calls**. That shape is the privacy story: if the only model call is a small one
that runs on the doctor's own machine, the patient's data never has to leave it.

This note ties together the two pieces of evidence that the local model can be
both **small enough** and **good enough** to be that part.

## 1. How small can the decomposer be? (`bench/`)

`bench/bench_models.py` swaps the decomposer across a model ladder and scores the
engine's diagnosis on the meningitis vignettes (full table in
`bench/BENCH_FINDINGS.md`). The findings:

- **The floor is set by the framework, not the weights.** A strict normalizer
  needs ~8B; teaching the deterministic normalizer to absorb the JSON shapes small
  models emit drops the full-score floor to **qwen2.5:1.5b (986 MB)** — a 5× smaller
  model. Intelligence accumulates in the framework so the model can be smaller.
- **Every size fails SAFE — 0 wrong diagnoses at any model size.** The closed-
  vocabulary gate drops any hallucinated finding before the engine sees it, so the
  catastrophic failure mode (a confident *wrong* answer) does not occur even at
  0.5B. The model either decomposes correctly or abstains.
- **Bigger is not better.** The largest model in the ladder (9.6 GB) scores *below*
  a 1 GB one — the axis that matters is structured-output discipline, not size.

## 2. Can a small model be *trained* into an expert decomposer? (`train/`)

Yes — and without human labels. `train/gen_data.py` has the **framework author its
own training data backward**: sample a finding-set from the dictionary (that *is*
the gold IR), have a teacher write natural prose for it, and the pair is
`(prose → that IR)`. Because we chose the findings, the label is exact; the teacher
only supplies language. A Gemma-3-1B LoRA trained on this went from **0/4 → 4/4** on
the vignettes (loss ~1.9 → ~0.02).

## The thesis

A small model that does **only** decomposition, made expert by data the framework
writes itself, plus a CPU engine that does all the reasoning over a grounded,
auditable rulebook — is enough to run the whole "messy human input → diagnosis (with
an audit trail) → what-to-check-next → constrained treatment/dosing" flow **locally,
at 0 answer-time model calls**. Correctness is enforced downstream by the grounded
rulebook and the closed-vocabulary gate, not by the model's size — which is exactly
why the model can be small enough to live on each doctor's machine.

## Where this goes next (roadmap C2/C3)

- **C2** — wire the trained specialist in as the production decomposer (local
  first, cloud fallback), so the entire warm path is on-device.
- **C3** — the ER spine: `mlx-whisper` voice → transcript → decompose → triage →
  immediate actions, fully local.

Decision-support only — never replaces a physician; the local flow hands over an
inspectable, overridable audit trail and the physician makes the call.
