# adjudication-clinical-demo (Rust)

Second-domain A/B demo. Same shape as `adjudication-tsa-demo`,
different source text. **Proves the framework's IR grammar +
checkers generalize across domains** — only the prompts and the
hand-built fixture change.

## Why two demos

The framework's design principle says intelligence accumulates in
the framework (ADJ02 coverage, ADJ03 polarity/modality, ADJ04
round-trip, ADJ05 adversary), not in any one domain prompt. This
crate is the verification: it reuses everything the TSA demo uses
and runs it against a clinical-triage assessment.

## Quick start

```bash
ollama serve   # in another terminal
ollama pull gemma4:latest
ollama pull llama3.1:8b    # for ADJ05

ADJ_DEMO_ENDPOINT=http://127.0.0.1:11434 \
  ADJ_DEMO_ADVERSARY_MODEL=llama3.1:8b \
  cargo run -p adjudication-clinical-demo
```

## The fixture

Source text (64 bytes):

```text
Patient: shortness of breath, mild fever, no known drug allergy.
```

Hand-built IR (`clinical_ir_document`):

| Node | Kind  | Polarity | Term                           | Source span             |
|------|-------|----------|--------------------------------|-------------------------|
| F1   | Fact  | Affirmed | `symptom(shortness_of_breath)` | `0..30`                 |
| F2   | Fact  | Affirmed | `symptom(fever, mild)`         | `30..42`                |
| F3   | Fact  | **Denied** | `drug_allergy(unknown)`      | `42..64`                |
| Q1   | Query | Affirmed | `safe_to_discharge(patient)?`  | _(synthesized)_         |

The **denied-polarity** allergy fact is the interesting part: a
small model often misses negation in source text ("no known drug
allergy" ≠ "drug allergy unknown"). ADJ03 catches this if the model
gets it wrong; the hand-built fixture demonstrates the right answer.

## Env vars

| Var                       | Default               | Purpose |
|---------------------------|-----------------------|---------|
| `ADJ_DEMO_ENDPOINT`       | `http://localhost:11434` | Ollama URL (use `127.0.0.1` on macOS) |
| `ADJ_DEMO_MODEL`          | `gemma4:latest`       | Primary model (Extractor/Renderer/Nli/Plausibility) |
| `ADJ_DEMO_ADVERSARY_MODEL`| _(none)_              | Second-family model for ADJ05 |
| `ADJ_DEMO_SOURCE`         | (canonical)           | Override the assessment text |
| `ADJ_DEMO_CACHE_DIR`      | _(none)_              | Disk-persisted prompt cache (huge speedup on re-runs) |
| `ADJ_DEMO_TIMEOUT_SECS`   | `120`                 | HTTP timeout |
| `ADJ_DEMO_AUDIT=1`        | _(off)_               | Dump the full ADJ07 audit trail |

## What v0.1 ships

- `DemoConfig` mirroring the TSA demo's shape.
- `run_raw_arm(cfg)` — single Ollama call with a triage-officer
  system prompt.
- `run_pipeline_arm(cfg)` — hand-built clinical IR + full pipeline.
- `clinical_ir_document(source)` — the fixture builder, with
  fallback for non-canonical text.
- 7 offline tests cover the IR shape, the denied-allergy polarity,
  span tiling, fallback behavior, default config, and the outcome
  formatter.

## What v0.1 deliberately does NOT do

- LLM-extracted IR mode. The TSA demo's v0.2-v0.4 work (full
  decompose_text flow + ADJ06 clarification loop) is reusable here
  but lands in a follow-up — v0.1 stays focused on the hand-built
  baseline that proves the pipeline generalizes.
- Real clinical domain knowledge. The IR shape is illustrative, not
  medically authoritative.
