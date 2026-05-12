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
# Default: 127.0.0.1:11434, model `gemma4:latest`, default TSA text.
ADJ_DEMO_ENDPOINT=http://127.0.0.1:11434 cargo run -p adjudication-tsa-demo

# Override the model:
ADJ_DEMO_MODEL=llama3.1:8b cargo run -p adjudication-tsa-demo

# Custom source text:
ADJ_DEMO_SOURCE='1 carry-on bag, lithium battery.' \
  cargo run -p adjudication-tsa-demo

# Dump the full ADJ07 audit trail:
ADJ_DEMO_AUDIT=1 cargo run -p adjudication-tsa-demo
```

## How the two arms work

### Arm A — raw model

Single `OllamaClient::complete` call with a system prompt that asks
the model to act as a TSA officer and emit a `VERDICT: COMPLIANT` /
`VERDICT: NON-COMPLIANT` line. No tooling, no checkers, no audit
trail. Whatever the model decides is what the user sees.

### Arm B — structured pipeline

Hand-builds the TSA fixture IR (same shape the ADJ10 integration
test uses), wraps the Ollama client in a `GatewayConfig` registered
against `Renderer` + `Nli`, and feeds the input through
`adjudication_pipeline::run_with_gateway`. The pipeline:

1. Records the document + IR nodes in an `AuditTrail` (ADJ07-v1).
2. Runs `adjudication-coverage::check_coverage` (ADJ02).
3. Runs `adjudication-polarity-modality::check_propagation` (ADJ03).
4. Runs `adjudication-round-trip::check_round_trip` (ADJ04) — calls
   `render_node` against `Renderer` and `entail` against `Nli` for
   each leaf node.
5. Records ADJ05 as `Skipped` (no second-family adversary yet).
6. Runs the logic engine against the IR if all gating checks passed.

The harness then prints both arms' outputs and (with `ADJ_DEMO_AUDIT=1`)
the full JSON audit trail.

## What v0.1.0 deliberately does NOT do

- **`decompose_text`** isn't called — the demo uses a hand-built IR
  because `adjudication-ir` doesn't yet derive `serde::Deserialize`,
  so the primitive's JSON output can't yet be converted into a typed
  `IRDocument`. Once those derives land, the demo can switch to a
  source-text-only entry point.
- **Second-model adversary** — the demo runs ADJ05 as `Skipped`. A
  future variant pulls a second model family (e.g.,
  `ollama pull llama3.1:8b`), registers it as `Role::Adversary`, and
  lets `check_adversarial` run.
- **Multi-document or multi-query inputs** — single TSA declaration
  with one query node.

## What you'll typically see

With a single 7-8B model serving every role and `gemma4:latest`'s
default behaviour, the raw arm often says `VERDICT: COMPLIANT` for
the default text *even though it contains a prohibited item*
(`matches`). The pipeline catches divergence:

- ADJ02 + ADJ03 pass cleanly (the IR is well-formed).
- ADJ04 may flag drift (the model's rendering of `prohibited(matches)`
  may not align with the source span) OR may fail with a schema
  validation error (smaller models sometimes return empty / malformed
  JSON for `entail`'s strict schema — the error is captured in the
  audit trail rather than silently swallowed).

The point isn't "the pipeline is always right" — it's "the pipeline
makes the model's failure modes inspectable." Both outcomes (drift
caught, schema validation tripped) leave a structured artifact a
reviewer can pick up.
