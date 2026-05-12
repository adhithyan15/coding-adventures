# Adjudication Framework — Overview

> A high-level map of how the pieces fit together. Read this before
> diving into the individual ADJ## specs.

## What this framework is

A pipeline that wraps a local LLM in **structured verification** so
the model produces auditable, replayable verdicts on domain-specific
questions. The design principle is the opposite of "scale the model
up": **shrink the model down, push intelligence into the framework**.

A 500-million-parameter model with the framework produces structured
output a reviewer can defend; the same model on its own confidently
hallucinates. Empirically verified — see
[ADJ12 small-model benchmarks](ADJ12-small-model-benchmarks.md).

## The user journey, end to end

```text
┌─────────────────┐    ┌─────────────────────────────────────────┐
│  source text    │───▶│  decompose_text (LLM, Role::Extractor)  │
└─────────────────┘    └────────────────┬────────────────────────┘
                                        │
                                        ▼
                       ┌─────────────────────────────────────────┐
                       │   typed IR document (ADJ01 grammar)     │
                       └────────────────┬────────────────────────┘
                                        │
                                        ▼
                       ┌─────────────────────────────────────────┐
                       │  ADJ02 coverage  (deterministic Rust)   │
                       │  ▶ EVERY byte covered? OR Discarded?    │
                       └────────────────┬────────────────────────┘
                                        │ pass
                                        ▼
                       ┌─────────────────────────────────────────┐
                       │  ADJ03 polarity/modality  (det. Rust)   │
                       │  ▶ Affirmed/Denied/Inherit consistent?  │
                       └────────────────┬────────────────────────┘
                                        │ pass
                                        ▼
                       ┌─────────────────────────────────────────┐
                       │  ADJ04 round-trip (LLM, Renderer + Nli) │
                       │  ▶ render IR back to text, NLI vs src   │
                       └────────────────┬────────────────────────┘
                                        │ pass / advisory
                                        ▼
                       ┌─────────────────────────────────────────┐
                       │  ADJ05 adversarial (LLM, Adversary)     │
                       │  ▶ different model finds alt readings   │
                       │  ▶ judge rules plausible/implausible    │
                       └────────────────┬────────────────────────┘
                                        │ pass / advisory
                                        ▼
                       ┌─────────────────────────────────────────┐
                       │  logic-engine  (deterministic Prolog)   │
                       │  ▶ structured query answer              │
                       └────────────────┬────────────────────────┘
                                        ▼
                       ┌─────────────────────────────────────────┐
                       │  ADJ07 audit trail JSON                 │
                       │  ▶ every checker, every LLM call,       │
                       │    prompt-hashed for replay             │
                       └─────────────────────────────────────────┘

  ◀── if ADJ02 fails ──┐
                       │
                       │ ADJ06 clarification dialogue:
                       │  re-prompt LLM with the structured
                       │  violation; model self-corrects;
                       │  re-run the pipeline. Up to N attempts.
```

## The two big asymmetries

1. **Expensive model, cheap checkers.** One LLM call costs more
   (tokens, latency, electricity) than a hundred deterministic
   Rust checks. The framework pushes work onto the cheap side
   wherever possible. ADJ02 + ADJ03 are pure Rust; ADJ04 + ADJ05
   are LLM but bounded; the engine is pure Rust.

2. **Force structural commitments.** Raw prompting lets the model
   pattern-match on the parts it understands and silently ignore
   the rest. The IR grammar forces the model to commit, for every
   byte of source, to one of `Fact / Query / Rule / Uncertainty /
   Exception / TextRun-grouping / Discarded(reason)`. No fourth
   option. The model can't hand-wave anymore.

## Crate map

### Foundation
- [`logic-core`](../packages/rust/logic-core) — `Term`, `LogicVar`,
  `Substitution`, `unify`. Term universe everything else operates
  on.
- [`logic-engine`](../packages/rust/logic-engine) — Prolog-style
  resolution engine that runs queries against a `KnowledgeBase` of
  facts and rules.

### IR + checkers (pure Rust)
- [`adjudication-ir`](../packages/rust/adjudication-ir) — ADJ01 v2
  IR grammar: `IRDocument`, `IRNode`, `Polarity`, `Modality`,
  `NodeKind`, `Span`, `validate`.
- [`adjudication-coverage`](../packages/rust/adjudication-coverage)
  — ADJ02 v2: every byte covered exactly once.
- [`adjudication-polarity-modality`](../packages/rust/adjudication-polarity-modality)
  — ADJ03 v2: polarity/modality propagation.

### LLM layer
- [`llm-gateway`](../packages/rust/llm-gateway) — `LlmClient` trait,
  `CompletionRequest`/`CompletionResponse`/`LlmError`. The
  provider-agnostic seam.
- [`llm-provider-ollama`](../packages/rust/llm-provider-ollama) —
  Ollama client over hand-rolled `std::net` HTTP/1.1, **zero
  third-party HTTP deps**. Handles chunked encoding, surfaces
  output truncation as a distinct error.
- [`llm-primitives`](../packages/rust/llm-primitives) — `decompose_text`,
  `render_node`, `entail`, `find_contradicting_reading`,
  `judge_plausibility`. Includes a thinking-mode-tolerant
  `complete_json_with_truncation_retry` helper.
- [`llm-cache`](../packages/rust/llm-cache) (v0.3) — content-
  addressed prompt cache with optional disk persistence. Turns
  a 60-second cold run into a 0.5-second warm replay.

### LLM-driven checkers
- [`adjudication-round-trip`](../packages/rust/adjudication-round-trip)
  — ADJ04: render each leaf back, NLI both directions, flag drift.
- [`adjudication-adversarial`](../packages/rust/adjudication-adversarial)
  — ADJ05: a DIFFERENT model proposes a contradicting reading; a
  judge rules plausibility.

### Orchestration + dialogue
- [`adjudication-pipeline`](../packages/rust/adjudication-pipeline)
  — `run_with_gateway` composes everything into a single call.
- [`adjudication-clarification`](../packages/rust/adjudication-clarification)
  — ADJ06: when ADJ02 fails, re-prompt the model with the
  structured violation and let it self-correct.

### Audit + replay
- [`adjudication-audit-trail`](../packages/rust/adjudication-audit-trail)
  — ADJ07: JSON-serializable trail with every document, every
  checker result, every LLM call (prompt-hashed for replay).
- [`adjudication-connector`](../packages/rust/adjudication-connector)
  — lowers IR rules/facts/queries into the logic-engine.

### Demos (each binary is a worked example)
- [`adjudication-tsa-demo`](../packages/rust/adjudication-tsa-demo)
  — TSA carry-on compliance. Both HandBuilt and LlmExtracted IR
  modes; full ADJ02+ADJ03+ADJ04+ADJ05+engine chain.
- [`adjudication-clinical-demo`](../packages/rust/adjudication-clinical-demo)
  — patient triage. Same machinery, different domain. Tests
  denied-polarity tracking ("no known drug allergy").
- [`adjudication-contract-demo`](../packages/rust/adjudication-contract-demo)
  — contract clause review. Tests rule+exception structure
  (`NodeKind::Rule` + `NodeKind::Exception` + `Conditional` modality).

## Running a demo locally

```bash
# Prereq: Ollama + two pulled models from different families.
ollama serve   # in another terminal
ollama pull gemma4:latest
ollama pull llama3.1:8b

# Default: TSA demo, HandBuilt IR, no cache.
ADJ_DEMO_ENDPOINT=http://127.0.0.1:11434 \
  cargo run -p adjudication-tsa-demo

# Full LLM-driven flow with adversary and disk cache:
ADJ_DEMO_ENDPOINT=http://127.0.0.1:11434 \
  ADJ_DEMO_ADVERSARY_MODEL=llama3.1:8b \
  ADJ_DEMO_IR_MODE=llm \
  ADJ_DEMO_CACHE_DIR=/tmp/adj-cache \
  cargo run -p adjudication-tsa-demo

# Try a 500M-parameter model (yes, this works):
ollama pull qwen2.5:0.5b
ADJ_DEMO_ENDPOINT=http://127.0.0.1:11434 \
  ADJ_DEMO_MODEL=qwen2.5:0.5b \
  ADJ_DEMO_ADVERSARY_MODEL=llama3.1:8b \
  ADJ_DEMO_IR_MODE=llm \
  cargo run -p adjudication-tsa-demo
```

## Key invariants the framework guarantees

1. **Total coverage.** ADJ02 v2 says every byte of normalized
   source belongs to some IR node — typed claim, structural
   grouping, or explicit `Discarded(reason)`. No silent skipping.
2. **Independence.** ADJ05 requires `(vendor, model_family)` for
   the Adversary client to differ from the Extractor client.
   Enforced by `GatewayConfig::check_independence`; the pipeline
   records `Skipped` with a typed reason if the check fails.
3. **Replayable audit.** Every primitive LLM call produces an
   `LlmCallRecord` with `prompt_version` + `prompt_hash`. Same
   prompt always produces the same response (temperature: 0.0);
   the cache is keyed on these. The audit trail captures the full
   conversation including ADJ06 dialogue turns.
4. **Token budget hygiene.** Truncation surfaces as
   `LlmError::OutputTruncated` (separate from `SchemaInvalid` for
   non-empty JSON). The retry helper doubles `max_tokens` up to a
   cap so thinking-mode models that burn their budget on
   chain-of-thought eventually succeed.
5. **Determinism by default.** Every primitive uses
   `temperature: 0.0`. The cache is sound because identical
   prompts produce identical responses. Disk persistence means a
   re-run replays at disk speed.

## What's deliberately out of scope today

- **Cloud LLM providers** (Anthropic, OpenAI). Need in-repo TLS
  1.3 first; deliberately queued. Ollama is the primary path.
- **ADJ06 for ADJ03/ADJ04/ADJ05.** v0.1 of clarification handles
  ADJ02 coverage failures only. Other violation kinds need their
  own prompt shapes; follow-ups.
- **Streaming responses.** All current primitives use single-shot
  JSON responses.
- **Multi-turn dialogue beyond Rung 1.** ADJ06 currently re-prompts
  the same model; Rung 2 (different model) and Rung 3 (human)
  are future work.

## Where to read next

- [ADJ00](ADJ00-adjudication-framework.md) — top-level framework spec.
- [ADJ01](ADJ01-adjudication-ir-grammar.md) — the IR grammar in detail.
- [ADJ02](ADJ02-coverage-checker.md) — coverage checker (the load-bearing one).
- [ADJ10](ADJ10-tsa-worked-example.md) — TSA worked example, fully
  derived.
- [ADJ12](ADJ12-small-model-benchmarks.md) — benchmark data on
  qwen2.5:0.5b through gemma4:8b.
- [LM00](LM00-llm-gateway-architecture.md) — gateway architecture.
- [LM00b](LM00b-llm-primitives.md) — primitives layer.
