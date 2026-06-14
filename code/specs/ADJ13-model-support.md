# ADJ13 — Model Support Matrix

> Per-model notes for picking which local model to use as each
> framework role. Captures empirical findings from running the
> three demos (TSA, clinical, contract) against multiple Ollama
> models.

## At a glance

| Model              | Params | Notes                                     | Recommended for                       |
|--------------------|--------|-------------------------------------------|---------------------------------------|
| `gemma4:latest`    | 8 B    | Strong thinking mode; verbose reasoning   | Extractor (esp. with v2 prompt)       |
| `gemma3:4b`        | 4 B    | (untested in framework; expected to work) | (probe with the TSA demo first)       |
| `llama3.1:8b`      | 8 B    | Family-disjoint from Gemma; clean JSON    | **Adversary** (the natural pairing)   |
| `qwen2.5:3b`       | 3 B    | Crisp JSON, fast, no thinking-mode tax    | Extractor; Renderer; Nli              |
| `qwen2.5:1.5b`     | 1.5 B  | Sometimes needs ADJ06 round; otherwise OK | Any role on constrained hardware      |
| `qwen2.5:0.5b`     | 0.5 B  | Surprisingly produces flat IR; **ADJ05 JSON shape can fail** | Extractor/Renderer; *not* Adversary |
| `mistral:7b`       | 7 B    | (untested in framework)                   | candidate for Adversary               |
| `phi-3:mini`       | 3.8 B  | (untested in framework)                   | candidate for any role                |
| `tinyllama:1.1b`   | 1.1 B  | (untested in framework)                   | candidate for stress-testing ADJ06    |

Models marked **untested** are likely-to-work because the framework
makes no model-family-specific assumptions; treat the table as a
starting point and probe with `cargo run -p adjudication-tsa-demo
ADJ_DEMO_MODEL=<name>`.

## Per-role guidance

### `Role::Extractor`

The most demanding role: must produce a flat-array IR conforming to
the v2 prompt (see `llm_primitives::decompose_text`). Notes:

- **`gemma4:latest`**: works, but its thinking-mode chain-of-thought
  burns 500–1000 output tokens before emitting JSON. The
  `complete_json_with_truncation_retry` helper handles this; cold
  latency lands at 30–60 s.
- **`qwen2.5:3b`**: ideal extractor for a single-laptop setup.
  Produces clean flat JSON without thinking-mode overhead; cold
  latency 20–30 s.
- **`qwen2.5:0.5b`**: produces a *valid* flat IR on the canonical
  TSA fixture. ADJ02 + ADJ03 pass. Surprising for a 500 M model.
- **`qwen2.5:1.5b`**: occasionally produces an IR with a 1-byte
  coverage gap. ADJ06 fires automatically and the model self-corrects
  on the second attempt. This is the load-bearing demonstration
  of the framework's "small model + retry-with-correction" thesis.

### `Role::Renderer`

Renders an IR node back into natural language for ADJ04 round-trip.
Any model that can produce a short coherent sentence works.

- All tested models pass.
- Latency dominates here when you have multiple IR leaves —
  every leaf is one round-trip.

### `Role::Nli`

Bidirectional entailment scoring (JSON shape with floats). Any
model with usable JSON mode works. Recommend the **same model as
Renderer** for cost reasons (you're already running it locally).

### `Role::Adversary`

ADJ05 requires a *different* `(vendor, model_family)` than
`Extractor`. Constraints:

- **Output must be valid JSON.** The 0.5 B Qwen sometimes emits
  malformed JSON for the `find_contradicting_reading` schema; the
  pipeline correctly surfaces this as `Failed` with the error in
  `telemetry.check_error`. Don't pick a model that can't produce
  reliable JSON.
- **Should be different family from Extractor.** If you've picked
  Gemma as Extractor, pick a Llama variant or Qwen variant as
  Adversary. The framework's `GatewayConfig::check_independence`
  enforces this.

Typical good pairings on a 16-GB laptop:

- `gemma4:latest` (Extractor) + `llama3.1:8b` (Adversary).
- `qwen2.5:3b` (Extractor) + `gemma4:latest` (Adversary) — costlier
  Adversary, cheaper Extractor.
- `qwen2.5:1.5b` (Extractor) + `llama3.1:8b` (Adversary) — for
  testing whether ADJ06 can rescue the smaller Extractor.

### `Role::Plausibility`

Binary plausibility judge for ADJ05. The demos use the same client
as Renderer/Nli (the primary model) — cost-effective and produces
sensible verdicts in practice. A purpose-trained judge model is a
future direction.

## Known failure modes per model size

### 7 B+ models
- Generally robust. The main hazard is thinking-mode token burn
  on extremely complex fixtures; `complete_json_with_truncation_retry`
  handles this by doubling the cap up to `MAX_TOKENS_CEILING`.

### 3 B models (qwen2.5:3b, phi-3:mini)
- Reliable for the demo fixtures. Some variance in renderer style
  across runs; cache the responses if you need reproducible outputs.

### 1.5 B models (qwen2.5:1.5b)
- Occasionally produces an IR with a 1-byte coverage gap on
  whitespace boundaries. ADJ06 catches this and re-prompts; usually
  resolves on attempt 2. Set `ADJ_DEMO_MAX_CLARIFY_ATTEMPTS=3` for
  extra headroom.
- May get polarity wrong on negation. ADJ03 catches this and
  blocks; a future ADJ06-for-ADJ03 will let the model self-correct.

### 0.5 B models (qwen2.5:0.5b)
- Surprisingly produces a clean flat IR for simple fixtures.
- May produce malformed JSON for the more complex `find_contradicting_reading`
  schema (which has nested optional fields). The pipeline surfaces
  this cleanly as a `Failed` ADJ05 with a parse error in telemetry,
  but you may want to skip ADJ05 entirely when using such a small
  Extractor.
- **Hallucination risk on the raw arm is high.** A 0.5 B model
  asked directly "is this passenger TSA-compliant?" will confidently
  invent rules (e.g., a 30-pound weight limit). The structured arm
  is the *only* trustworthy output at this scale.

## How to probe a new model

```bash
ollama pull <model>

# Smoke-test the extractor role:
ADJ_DEMO_ENDPOINT=http://127.0.0.1:11434 \
  ADJ_DEMO_MODEL=<model> \
  ADJ_DEMO_IR_MODE=llm \
  cargo run -p adjudication-tsa-demo

# Check the ADJ02/ADJ03 outcomes in the report. If ADJ02 fails,
# does ADJ06 fire and recover? If ADJ03 fails, the model has a
# polarity/modality issue that ADJ06-for-ADJ03 (future) will help with.

# Then pair with an adversary from a different family:
ADJ_DEMO_ENDPOINT=http://127.0.0.1:11434 \
  ADJ_DEMO_MODEL=<extractor-model> \
  ADJ_DEMO_ADVERSARY_MODEL=<adversary-model> \
  ADJ_DEMO_IR_MODE=llm \
  cargo run -p adjudication-tsa-demo

# If ADJ05 errors with a JSON parse, the adversary model can't
# produce reliable JSON for find_contradicting_reading. Drop down
# to a different adversary model, or accept ADJ05 Skipped.
```

## Future benchmark work

- Cross-domain × cross-model matrix: 3 demos × all tested models.
- ADJ06-for-ADJ03 retry: would let small models recover from
  polarity errors the way they already recover from coverage gaps.
- Cost / energy measurements per role per model (currently
  `LlmCallRecord::cost_usd` is hard-coded to 0.0 for Ollama; a
  follow-up could plumb a per-model rate table).
