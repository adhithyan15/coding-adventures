# Changelog

All notable changes to this project will be documented in this file.

## [0.10.0] - 2026-05-12 — Adversarial multi-model elicit-mode wiring

### Added

`ADJ_DEMO_RULEBOOK_MODE=adversarial:model1,model2,...` — elicit
rulebooks from N independent models and inject the
provenance-tagged merged text into Arm A's system prompt. Builds
on `adjudication_rulebook::acquire_rulebook_adversarial` (added in
that crate's v0.2) and the existing elicit-mode plumbing (v0.9).

The demo binary parses the comma-separated model list, builds one
OllamaClient + GatewayConfig per model, and dispatches the
adversarial elicit. Each model's outcome is logged with a checkmark
or cross plus byte count and validation status:

```text
[stage 0] adversarial elicitation across 2 models: gemma4:latest, llama3.1:8b
[stage 0] adversarial elicit: 2/2 models succeeded (0 failed)
[stage 0]   ✓ `gemma4:latest`: 1938 bytes (validation=FAILED (...))
[stage 0]   ✓ `llama3.1:8b`: 1709 bytes (validation=FAILED (...))
```

A reviewer reading Arm A's answer can grep the cited rule back to
the `=== RULEBOOK FROM <model> ===` section header in the audit
trail and see which model produced it. A rule appearing in only
one model's section is lower-trust than one in both.

`ADJ_DEMO_DUMP_RULEBOOK=1` works in adversarial mode too — dumps
the full provenance-tagged merged rulebook between markers.

### Why this matters

ADJ15's empirical results showed single-model recursive elicitation
flips verdicts at 3B+ but every elicited rulebook had at least one
fabrication (the matches-as-flammable-materials leap, the
fabricated doctor's-note exception, the wrong knife-length limit).
Multi-model elicitation reduces this risk: a fabricated rule that
appears in only one model's training shows up as a single-source
rule in the merged text, where reviewers can spot it. Rules
multiple models produced independently are higher trust.

24 unit tests still pass. Demo binary smoke-tested with
`adversarial:gemma4:latest,llama3.1:8b`.

## [0.9.0] - 2026-05-12 — elicit-mode wiring + visibility fixes

### Added

- **`ADJ_DEMO_RULEBOOK_MODE=elicit`** — the binary's main loop now
  calls `adjudication_rulebook::acquire_rulebook` against the
  configured model BEFORE Arm A runs, then injects the elicited
  rulebook text into Arm A's system prompt. This is the recursive
  use of the framework: the model is judged against rules it just
  produced from its own weights, audited by the same checker
  discipline that protects extracted facts.

  Flow when `ADJ_DEMO_RULEBOOK_MODE=elicit`:
  ```text
  [stage 0] elicit_rules + decompose_text + validate
            → Tentative Rulebook + audit_trail
  [arm A]   run_raw_arm(cfg with rulebook_text=Some(rb.source_text))
  [arm B]   run_pipeline_arm — unchanged
  ```

- **`ADJ_DEMO_DUMP_RULEBOOK=1`** — companion to elicit-mode. When
  set, prints the elicited rulebook text to stdout between
  `----- BEGIN RULEBOOK -----` and `----- END RULEBOOK -----`
  markers. Useful for reviewers who want to see what each model
  produced.

### Fixed

- **Stage 0 failures now print to stdout too**. Previously the
  elicitation failure message went only to stderr, which benchmark
  scripts that capture only stdout treated as silent success. Now
  the failure log line shows up in both streams; benchmark scripts
  can grep stdout for `"[stage 0] rulebook elicitation FAILED:"`.
- The success-path log line now reports the **validation
  diagnostic** when `validation_passed = false`, not just the
  boolean — so reviewers see *why* the rulebook IR failed
  `adjudication_ir::validate` (most commonly a coverage gap or a
  schema-invalid response from the model).

### Note on the qwen2.5:1.5b "silent" failure observed in ADJ15

The first ADJ15 benchmark run flagged a "silent failure" on
qwen2.5:1.5b — the demo binary's stage-0 success log line was
missing but no error was visible. After re-running with the
visibility fix, the failure mode is now clear: the model **collapses
into degenerate repetition** during the rulebook decompose phase,
producing 90+ identical IR nodes (`{"functor": "authority", "args":
[{"atom": "tsoa"}]}`) until it exhausts the output-token budget
mid-string. `complete_json_with_truncation_retry` correctly
surfaces `Gateway(SchemaInvalid { ... EOF while parsing ... })`;
the demo prints the typed error and falls back to no-rulebook
mode. The framework's behaviour is correct (graceful degradation,
clear diagnostic); the failure itself is an upstream small-model
coherence issue, not a framework bug. Recorded for follow-up:
investigating prompt-level anti-repetition mitigations and / or
explicit `max_nodes` caps in `decompose_text`.

### Unused-mut warning fixed

`json_to_ir_flat`'s `node` binding doesn't need `mut` after the
v3 cascade dropped the `part_of` re-write logic. Cleanup.

## [0.7.0] - 2026-05-12

### Added

ADJ06 clarification loop now also covers ADJ03 polarity/modality
failures, not just ADJ02 coverage failures.

- Each iteration of the loop picks the FIRST failing gating check
  and dispatches to the matching retry primitive:
  - ADJ02 coverage   → `retry_decompose_on_coverage_failure`
  - ADJ03 polarity   → `retry_decompose_on_polarity_failure`
- The framework auto-synthesizes a polarity hint when the source
  text contains a common negation token (e.g., "no", "not",
  "never", "without", "denies", "denied", "absent", "ruled out").
  Helps the model recognise that nodes covering negated text should
  be `Denied`, not `Affirmed`.
- The summary line in the side-by-side report now reports WHICH
  checker the loop ended on:
  - `"N rounds — resolved (ADJ02 + ADJ03 both pass)"`
  - `"N rounds — exhausted (ADJ02 fixed, ADJ03 still failing)"`
  - `"N rounds — exhausted (ADJ02 still failing)"`
- New `format_first_adj03_violation` + `build_polarity_hint`
  helpers next to the existing ADJ02 ones.

### Why

The ADJ12 small-model benchmark showed qwen2.5:1.5b recovering
from ADJ02 via clarification, then getting stuck on ADJ03 (likely
on the polarity of "matches"). With v0.7 of the demo + v0.2 of
the clarification crate, the same 1.5B model can now self-correct
on both gating checks within the same `max_clarification_attempts`
budget.

## [0.6.0] - 2026-05-12

### Added

`PipelineArmReport::cache_stats` — aggregate `CacheStats` across
every role's `CachingClient`. Side-by-side report prints a
`cache: N hits / M misses (X% hit rate), K entries` line whenever
any cache activity happened, making the cache's economic value
visible.

Measured against the canonical TSA fixture (gemma4 + llama3.1):

- Cold run: `2 hits / 9 misses (18%)` — the two hits come from
  `render_node` being called by both ADJ04 and ADJ05 with the same
  prompts, so the second checker hits the in-memory cache.
- Warm run with persisted cache: `11 hits / 0 misses (100%)`.

## [0.5.0] - 2026-05-12

### Added

Optional disk-persisted prompt cache. When `ADJ_DEMO_CACHE_DIR=<path>`
is set, every LLM client used by Arm B is wrapped in
`CachingClient::with_disk_persistence`. A first run populates the
cache directory; a second run replays every prompt from disk with
zero round-trips to Ollama.

Verified on the local TSA demo:

- Cold run (gemma4:latest extractor + renderer + nli, llama3.1:8b
  adversary, 9 LLM calls total): ~60 s wall-clock.
- Warm run with the same cache dir: ~0.5 s of actual demo work
  (rest is Rust binary launch).

- `DemoConfig::cache_dir: Option<String>` — opt-in. Defaults to
  `None` (in-memory cache only, which is essentially a no-op for a
  one-shot binary).
- `ADJ_DEMO_CACHE_DIR=<path>` env var to enable disk persistence
  from the binary.
- Side-by-side report header surfaces the cache configuration.

### Notes

ADJ06 retry rounds also benefit: when the model self-corrects an
ADJ02 failure, the corrected prompt is cached so a re-run replays
both the original failed attempt AND the correction at disk speed.

## [0.4.0] - 2026-05-12

### Added

ADJ06 clarification dialogue wired into the LlmExtracted flow. When
the model's first IR fails ADJ02 coverage, the demo re-prompts the
model with the structured violation and the model's previous output,
then re-runs the entire pipeline. This is the **self-correction
loop** that turns small local models into reliable extractors.

- `DemoConfig::max_clarification_attempts: usize` (default 2). Set
  via `ADJ_DEMO_MAX_CLARIFY_ATTEMPTS=N`. `0` disables clarification
  entirely (the v0.3 behaviour).
- `IrSourceTelemetry::LlmExtracted` gains `clarification_summary:
  Option<String>` and `clarification_turns: Vec<DialogueTurn>`. The
  side-by-side report shows the summary (`"1 round (resolved)"` /
  `"2 rounds (exhausted)"`); the audit-trail dump includes the full
  dialogue.
- The pipeline output's `audit_trail.dialogue` is populated with the
  retry turns, so ADJ07 captures the conversation end-to-end.

### What the loop does

1. Initial decompose_text → IR → pipeline.
2. If ADJ02 passed: done.
3. Otherwise: extract the first ADJ02 violation, call
   `retry_decompose_on_coverage_failure` with the prior IR JSON +
   violation description, get a corrected IR back, re-run the full
   pipeline.
4. Repeat up to `max_clarification_attempts` times.

The loop is dormant in the v2-prompt happy path — gemma4 produces a
clean IR on the canonical fixture and ADJ02 passes on the first try.
ADJ06 stands by for the harder cases (longer documents, more
ambiguous spans, smaller models).

## [0.3.0] - 2026-05-12

### Added

ADJ05 adversarial check now actually runs when a second model is
registered. v0.2 had ADJ05 hard-Skipped; v0.3 plumbs a real Adversary
client through the pipeline. With `gemma4:latest` as Extractor/
Renderer/Nli and `llama3.1:8b` as Adversary, the pipeline runs the
full chain ADJ02 + ADJ03 + ADJ04 + ADJ05.

- `DemoConfig::adversary_model: Option<String>` — opt-in. Defaults to
  `None`, which preserves the v0.2 "ADJ05 records Skipped" behaviour.
- `ADJ_DEMO_ADVERSARY_MODEL=llama3.1:8b` — env var to flip it on at
  the binary level.
- The pipeline now registers four roles: Extractor, Renderer, Nli,
  Plausibility (all served by the primary model) plus Adversary
  (served by the second model).
- ADJ05 findings (`Adj05AdversarialFinding`) are surfaced in the
  side-by-side report alongside ADJ04 drift findings. Each entry
  prints the IR's rendering, the adversary's contradicting reading,
  the adversary's explanation of how they differ, and the judge's
  reason for ruling the alternative reading plausible.
- ADJ05 skipped/errored telemetry (e.g., independence violation,
  missing role) surfaced in the report instead of disappearing.

## [0.2.0] - 2026-05-11

### Added

The **full LLM-driven flow** end-to-end. Where v0.1 hand-built the
TSA IR programmatically, v0.2 adds an `IrMode::LlmExtracted` mode
that calls `llm_primitives::decompose_text` to ask the model to
produce the IR, then converts the JSON output into a typed
`IRDocument` via a tolerant parser. The pipeline then runs ADJ02 +
ADJ03 + ADJ04 over the model-generated IR.

- `IrMode { HandBuilt, LlmExtracted }` — selected by
  `ADJ_DEMO_IR_MODE=hand|llm`. Defaults to `HandBuilt` for the
  clean-baseline experience; `llm` drives the full flow.
- `IrSourceTelemetry` recorded on `PipelineArmReport` so the printed
  report shows the IR's provenance (hand-built, LLM-extracted with
  the raw JSON + converter warnings, or LLM-extraction-failed with
  the fallback path).
- `json_to_ir_document` — the tolerant converter. Accepts both
  `kind` and `node_type`, both `term` and `text` field names, walks
  nested `children` arrays (Gemma 4 emits a tree, not the flat list
  the prompt asks for), defaults missing kind/polarity/modality to
  safe neutrals, clamps out-of-bound spans to source length, skips
  degenerate spans, synthesizes a `compliant(passenger_a)?` Query
  node if the LLM omits queries. Every fallback is logged in a
  `warnings` vector and surfaced in the report.
- Side-by-side report now surfaces:
  - The IR's source (hand-built vs LLM-extracted vs LLM-failed) +
    converter warning count.
  - **ADJ02 coverage violations** when the pipeline blocks (e.g.,
    `RootsDoNotTileDocument { missing_ranges: [(2, 3)] }` when the
    model's IR has a 1-byte gap at the space between "1" and
    "carry-on").
  - **ADJ04 round-trip drift findings** with quantified NLI scores
    in both directions (e.g., `source→rendering = 0.95`,
    `rendering→source = 0.10` when the model adds claims not in the
    source).
- `ADJ_DEMO_IR_MODE` env var documented in `main.rs`.
- `ADJ_DEMO_AUDIT=1` now also dumps the LLM-extracted IR (raw JSON
  from `decompose_text`) and the converter warnings before the
  audit-trail JSON.

13 new unit tests cover the converter (well-formed shape, clamping,
default kind, atom term, root-not-object rejection, nodes-array
required, degenerate-span skip, missing-spans fallback,
empty-spans-OK-for-Query, nested-children flattening, node_type
alias, text-as-term fallback, query synthesis).

### Verified end-to-end against `gemma4:latest`

- **Hand-built mode**: ADJ02 + ADJ03 pass; ADJ04 catches the model's
  renderer drifting from source on both Facts (`"1 carry-on bag, "`
  → `"The carry-on bag is available."`, NLI scores 0.10/0.10). Engine
  runs.
- **LLM-extracted mode**: decompose_text produces a nested-tree IR
  with spans 0..2 and 3..24. The converter flattens it. ADJ02 catches
  the 1-byte gap at byte 2 (the space between "1" and "carry-on")
  as `RootsDoNotTileDocument { missing_ranges: [(2, 3)] }`. Pipeline
  Blocks before the engine runs — token-burn avoided.

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
