# Training the local specialist decomposer

The warm path asks the local model for exactly **one** thing: turn messy clinical
prose into typed findings in the closed dictionary. Everything downstream is the
same 0-answer-time-model-call CPU engine over the grounded rulebook. So the model
is a small, swappable part — and the goal is to make a model that does *only*
decomposition, as **small and fast** as possible, so it runs on a doctor's own
machine and the patient's data never leaves it (privacy / HIPAA by architecture).

This directory trains that specialist with MLX LoRA on Apple Silicon.

## The idea: the framework authors its own training data (backward generation)

To make a small model an expert decomposer we need many `(prose → IR)` pairs with
**perfect** labels. Distilling a teacher's *extraction* would inherit the
teacher's errors. So `gen_data.py` runs the generator **backward**:

1. sample a finding-set from the dictionary — this **is** the gold IR, by
   construction;
2. ask a teacher model to write natural clinical prose stating exactly those
   findings (varied phrasing, optional noise sentence);
3. the training pair is `(decompose-prompt + prose) → (the sampled IR)`.

Because we *chose* the findings, the label is exact — the teacher only supplies
language, never the ground truth. The mix deliberately includes bacterial / viral
/ mixed profiles, negations, and **ABSTAIN** cases (prose with no dictionary
findings → empty IR) so the model learns to decline rather than hallucinate.

### The gold IR carries byte-provenance, discard, and inference justification

The label is not a bare finding list — it is the full typed IR the warm pipeline's
prompt asks for, derived **deterministically from the teacher's prose** by
`build_gold_ir()` (pure, no model — so it is unit-tested without Ollama):

- **byte-provenance** — each finding records the verbatim `span` of prose that
  supports it (located by matching the finding's dictionary surface forms against
  the vignette). A finding stated verbatim → `type: stated`; one the teacher
  paraphrased past recognition → `type: inferred` with an empty span.
- **inference justification** — each finding gets an `ENTAILED` verdict when its
  span was found verbatim, or `LEAP` when it was not (which `ir_to_adj` then drops —
  the safe behavior: the model is taught to mark, not fabricate, unstated findings).
- **discard** — a third of vignettes carry an injected non-diagnostic **distractor**
  (a vital sign, social-history detail, symptomatic med). When it lands in the
  prose, the gold records it as a `discard` `{span, reason}`, teaching the model to
  set a red herring aside *with a justification* rather than coin a finding from it.

So the model learns the discipline the framework demands of itself: extract only
what the bytes support, cite the span, and justify both what it keeps and what it
discards. The gold finding still keeps `functor`/`value`/`polarity` (what the
rulebook consumes), so the addition is non-breaking downstream.

## Workflow

```sh
# 1. author the data (teacher writes prose for sampled finding-sets)
python3 gen_data.py --n 300 --teacher llama3.1:8b --seed 0   # → data/train.jsonl, data/valid.jsonl

# 2. LoRA fine-tune a small base with mlx-lm (example: Gemma-3-1B)
mlx_lm.lora --model mlx-community/gemma-3-1b-it-4bit --train \
    --data data --adapter-path adapters --iters 400 --mask-prompt

# 3. score base vs base+LoRA through the SAME framework (ir_to_adj → decide)
python3 eval_specialist.py --model mlx-community/gemma-3-1b-it-4bit                     # base
python3 eval_specialist.py --model mlx-community/gemma-3-1b-it-4bit --adapter adapters  # specialist
```

`eval_specialist.py` runs the model as the warm-path decomposer and scores the
engine's diagnosis vs gold — identical scoring to `../bench/bench_models.py`, so
base-vs-specialist is apples-to-apples. The model never diagnoses; it only
decomposes.

## Result

Training the framework-authored data took the base model from **0/4 → 4/4** on the
meningitis vignettes (training loss ~1.9 → ~0.02) — a small local model made an
expert decomposer by data the framework wrote itself, with no human labels. See
`../bench/BENCH_FINDINGS.md` for how small the *base* models can go with a tolerant
framework (down to ~1 GB), and `../LOCAL-MODEL-FINDINGS.md` for the privacy thesis.

## Second IR shape: the chart-fact decomposer (`gen_chart_data.py`)

The decomposer's job is to turn *any* messy clinical input into a typed IR. `gen_data.py`
teaches the **findings** shape (CSF labs / organism-id → diagnosis). `gen_chart_data.py`
teaches the **chart-fact** shape: a free-text patient chart note → the typed
`ChartFact{kind, value, span}` list the **chart-as-constraints COP** consumes
(`../treatment/antibiotics/chart_to_cop.py`). The structured path already exists
(`../fhir/fhir_to_chartfacts.py` maps a FHIR bundle → ChartFacts); this is its **prose
counterpart** — the messy-input front door to the constraint solver (the CC-7 enabler).

Same backward-generation + byte-provenance discipline: sample a chart-fact set from the
**closed** vocabulary (exactly what `compile_cop` maps), a teacher writes a chart note
stating them (+ a non-charting distractor), and the gold IR is derived from the note with
verbatim spans + a justified `discard` list. The headline guarantee
(`test_gen_chart_data.py`) is the **F3→F2 consumability contract**: every `(kind, value)`
the generator can sample is fed through `compile_cop` and asserted *not discarded* — so the
decomposer's gold can never contain a chart fact the COP would silently drop (closed-vocab
adherence proven against the actual downstream consumer, not just a schema).

```sh
python3 gen_chart_data.py --n 200 --teacher llama3.1:8b   # → data/chart_{train,valid}.jsonl
python3 test_gen_chart_data.py                            # 6 pure tests (no Ollama/MLX)
```

## What is and isn't committed

Source only: `gen_data.py`, `gen_chart_data.py`, `eval_specialist.py`, this README. The
`.gitignore` excludes `.venv/`, `data/` (generated pairs), `adapters/` (trained weights),
and `*.log` — those are reproducible from the scripts + a base model, and the weights
are large/environment-specific. Regenerate with the workflow above.

## Honest limits

- 4 vignettes, one differential — directional, not a powered benchmark.
- The trained adapter overfits a narrow task by design (decompose *this*
  dictionary); it is a proof that the specialist approach works, not a shippable
  clinical model.
- Requires `mlx-lm` (Apple Silicon) for training and `ollama` for the teacher.
