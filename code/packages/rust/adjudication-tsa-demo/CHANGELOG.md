# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-05-11

### Added

A/B comparison demo: raw Ollama model vs the full adjudication
pipeline, both fed the same TSA carry-on declaration. Ships as a
library + binary; the binary is the headline
`cargo run -p adjudication-tsa-demo` entry point.

- `DemoConfig { endpoint, model, timeout, source_text }` — the
  configuration knob. Reads from `ADJ_DEMO_ENDPOINT`,
  `ADJ_DEMO_MODEL`, `ADJ_DEMO_SOURCE`, `ADJ_DEMO_TIMEOUT_SECS`.
- `run_raw_arm(&cfg)` — single `OllamaClient::complete` call with a
  TSA-officer system prompt. Returns `RawArmReport` with the answer
  + token counts + latency.
- `run_pipeline_arm(&cfg)` — wraps the Ollama client in a
  `GatewayConfig` against `Role::Renderer` + `Role::Nli` and runs
  `adjudication_pipeline::run_with_gateway` over a hand-built TSA
  IR. Returns `PipelineArmReport` with the four checker outcomes,
  the verdict summary, and the full `PipelineOutput`.
- `tsa_ir_document(&source_text)` — builds the canonical
  `1 carry-on bag, matches.` IR (two facts tiling the document plus
  one query); falls back to a single-fact IR for arbitrary text.
- `format_side_by_side(&raw, &pipeline)` — renders both arms into a
  multi-line human-readable report.
- 6 offline unit tests cover: default text yields the canonical
  three-node IR, non-default text yields fallback IR, empty source
  yields the query-only IR, raw prompt embeds source text + verdict
  cue, default config targets the local Ollama port, the outcome
  formatter handles each variant.

### Notes

- `decompose_text` is intentionally not wired into Arm B yet —
  `adjudication-ir` does not derive `serde::Deserialize`, so the
  primitive's `serde_json::Value` output can't be converted to a
  typed `IRDocument`. The demo uses a hand-built IR until those
  derives ship.
- ADJ05 stays Skipped because the demo uses one model family for
  every role. Pulling a second model and registering it as
  `Role::Adversary` flips ADJ05 from Skipped to Passed/Failed.
- On macOS, default to `ADJ_DEMO_ENDPOINT=http://127.0.0.1:11434`
  rather than `localhost`. The hostname `localhost` often resolves
  to `::1` (IPv6) on macOS and Ollama binds only IPv4, which
  produces `Connection refused` from the bespoke HTTP client.
