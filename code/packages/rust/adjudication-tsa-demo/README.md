# adjudication-tsa-demo (Rust)

A/B comparison: a raw local Ollama model versus the full adjudication
pipeline, both fed the same TSA carry-on declaration. Prints a
side-by-side report.

## Why this crate exists

The adjudication framework wraps an LLM in a chain of checkers
(coverage, polarity/modality, round-trip, adversarial) plus a
deterministic logic engine. The natural question is: *what does the
pipeline actually buy you over just asking the model?*

This demo answers it by running both arms on the same input and
reporting what each one says.

## What you need

- A local Ollama instance reachable at `http://127.0.0.1:11434`
  (`ollama serve`).
- One model pulled. The default config targets `gemma4:latest`, but
  any chat model the local Ollama serves will do.

`localhost` is intentionally avoided in the default config because
on macOS `localhost` often resolves to `::1` (IPv6), and Ollama
binds only the IPv4 socket — connect refused. Use `127.0.0.1`.

## Quick start

```bash
# Default: 127.0.0.1:11434, model `gemma4:latest`, default TSA text,
# hand-built IR (clean baseline).
ADJ_DEMO_ENDPOINT=http://127.0.0.1:11434 cargo run -p adjudication-tsa-demo

# Full LLM-driven flow: model produces the IR, pipeline checks it:
ADJ_DEMO_ENDPOINT=http://127.0.0.1:11434 \
  ADJ_DEMO_IR_MODE=llm \
  cargo run -p adjudication-tsa-demo

# Override the model:
ADJ_DEMO_MODEL=llama3.1:8b cargo run -p adjudication-tsa-demo

# Custom source text:
ADJ_DEMO_SOURCE='1 carry-on bag, lithium battery.' \
  cargo run -p adjudication-tsa-demo

# Dump the full ADJ07 audit trail + the LLM's raw IR + converter warnings:
ADJ_DEMO_AUDIT=1 ADJ_DEMO_IR_MODE=llm cargo run -p adjudication-tsa-demo
```

## How the two arms work

### Arm A — raw model

Single `OllamaClient::complete` call with a system prompt that asks
the model to act as a TSA officer and emit a `VERDICT: COMPLIANT` /
`VERDICT: NON-COMPLIANT` line. No tooling, no checkers, no audit
trail. Whatever the model decides is what the user sees.

### Arm B — structured pipeline

Wraps the Ollama client in a `GatewayConfig` registered against
`Role::Extractor` + `Role::Renderer` + `Role::Nli`, and feeds the
input through `adjudication_pipeline::run_with_gateway`. The
pipeline:

1. Builds the IR. Two modes:
   - `IrMode::HandBuilt` (default): the demo constructs the canonical
     TSA fixture IR programmatically. Useful as a clean baseline
     that proves the pipeline machinery works.
   - `IrMode::LlmExtracted` (`ADJ_DEMO_IR_MODE=llm`): calls
     `llm_primitives::decompose_text` to ask the model to produce
     the IR, then converts the JSON output to a typed `IRDocument`
     via a tolerant parser. This is the *full* LLM-driven flow.
2. Records the document + IR nodes in an `AuditTrail` (ADJ07-v1).
3. Runs `adjudication-coverage::check_coverage` (ADJ02).
4. Runs `adjudication-polarity-modality::check_propagation` (ADJ03).
5. Runs `adjudication-round-trip::check_round_trip` (ADJ04) — calls
   `render_node` against `Renderer` and `entail` against `Nli` for
   each leaf node.
6. Records ADJ05 as `Skipped` (no second-family adversary yet).
7. Runs the logic engine against the IR if all gating checks passed.

The harness then prints both arms' outputs, the IR's provenance, any
ADJ02 coverage violations, any ADJ04 round-trip drift findings, and
(with `ADJ_DEMO_AUDIT=1`) the full JSON audit trail plus the LLM's
raw `decompose_text` output and converter warnings.

## Tolerant JSON-to-IR converter

`json_to_ir_document` is intentionally forgiving:

- Accepts both `kind` and `node_type` (Gemma 4 prefers `node_type`).
- Accepts both `term` and `text` (Gemma 4 prefers `text`; the
  converter wraps a `text` string in a `text_claim/1` compound).
- Walks nested `children` arrays — when the model emits a tree
  instead of the flat list the prompt asks for, every leaf becomes
  an IR node and the grouping parents are dropped.
- Missing `kind` defaults to `Fact`. Missing `polarity`/`modality`
  default to `Affirmed`/`Present`.
- Missing `source_spans` falls back to a single span covering the
  whole document for non-Query kinds; Query nodes are allowed to
  have zero spans per ADJ02 v2.
- Out-of-bound span ends are clamped to source length.
- Degenerate spans (`end <= start`) are skipped.
- If the IR has no Query node, the converter synthesizes
  `compliant(passenger_a)?` so the engine has something to run.

Every fallback is recorded as a warning surfaced in the report.

## What v0.2.0 ships

- Both IR modes (`HandBuilt` / `LlmExtracted`).
- The tolerant JSON-to-IR converter with 13 unit tests.
- Side-by-side report with IR provenance + ADJ02 + ADJ04 findings.
- Optional audit-trail dump with model's raw IR + converter warnings.

## What v0.2.0 deliberately does NOT do

- **Second-model adversary** — the demo runs ADJ05 as `Skipped`. A
  future variant pulls a second model family (e.g.,
  `ollama pull llama3.1:8b`), registers it as `Role::Adversary`, and
  lets `check_adversarial` run.
- **Multi-document or multi-query inputs** — single TSA declaration.

## What you'll typically see

Against `gemma4:latest` with the default text `1 carry-on bag, matches.`:

- **Arm A** says `VERDICT: COMPLIANT` — confidently ignores that
  `matches` are a prohibited carry-on item.
- **Arm B hand-built mode**: ADJ02 + ADJ03 pass; ADJ04 catches the
  model's renderer drifting from source on both Facts (NLI scores
  around 0.10 in both directions for added/changed claims). Engine
  runs.
- **Arm B LLM-extracted mode**: decompose_text produces a nested
  tree. The converter flattens it. ADJ02 catches a 1-byte coverage
  gap at byte 2 (the space between "1" and "carry-on") as
  `RootsDoNotTileDocument { missing_ranges: [(2, 3)] }`. Pipeline
  Blocks before the engine runs — no token-burn.

The point isn't "the pipeline is always right" — it's that **the
pipeline makes the model's failure modes inspectable.** Whether the
model is confidently wrong, or its IR has a 1-byte hole, or its
rendering drifts from the source, the audit trail records a
structured artifact a reviewer can pick up.
