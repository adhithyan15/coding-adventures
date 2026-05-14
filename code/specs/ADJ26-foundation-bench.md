# ADJ26 — Foundation Bench: Hierarchical Decomposition × Per-Level Coverage

> **Methodology PR. Empirical results follow in a data PR.**
>
> ADJ25's migration plan calls this PR-6, the foundation bench. It is
> the first empirical measurement of whether the hierarchical
> decomposition flow (Document → Sentence → Phrase → Claim →
> TypedComponent) works against real LLMs.
>
> The bench is the gate that unblocks the rest of the framework:
> until it shows reliable per-level coverage across the 8-declaration
> × 5-model matrix, the paused workstreams (ADJ14/15/17 rulebook
> elicitation, ADJ16 engine adjudication, ADJ18/19 verdict benches)
> stay paused.

## What this bench measures

For each `(declaration, model)` cell:

1. Run `decompose_hierarchical` against the source text via a real
   Ollama gateway. The orchestrator walks the four level boundaries
   in order, dispatching one LLM call per parent at each level, then
   runs `check_hierarchical_coverage` and retries on every failing
   parent up to `max_retries_per_parent`.
2. Capture per-level coverage pass/fail. Each boundary produces an
   `passed: bool` + `gap_count: usize`.
3. Capture flattening violations separately so we can see how often
   the LLM tries to smuggle source content into atom names
   (`50_wh`, `4_inch_blade`, etc.).
4. Capture `check_correlation_completeness` outcome on the
   orchestrator's final IR — should be `pass` by construction (PR-5
   guarantees this), so any failure here is a wiring bug.
5. Wallclock latency.
6. Total LLM calls dispatched (initial + retries).

The headline metric is **per-level coverage pass rate**:

```
% cells where ADJ02-style coverage held at every parent → child
boundary across the four levels.
```

A cell counts as "passing" only when:
- `Document → Sentence` tiled exactly,
- AND `Sentence → Phrase` tiled exactly at every sentence,
- AND `Phrase → Claim` tiled exactly at every phrase,
- AND `Fact → TypedComponent` tiled exactly at every fact,
- AND no atom in the IR violated the no-flattening rule.

This is the "every byte accounted for" hypothesis from
[project_total_coverage_forces_reasoning](../../memory/project_total_coverage_forces_reasoning.md),
operationalised. ADJ12 measured the flat-tile version on a single
fixture; ADJ26 measures the hierarchical version across the
8-declaration set ADJ18 established.

## What this bench deliberately does NOT measure

- **Verdict accuracy.** ADJ18 measured Arm A (raw LLM verdict).
  ADJ26 measures *Arm B*'s decomposition step only. A cell that
  produces a perfectly-tiled hierarchical IR but the verdict
  downstream is wrong still counts as "passing" for ADJ26.
  Verdict accuracy returns to scope after the foundation gate
  holds.
- **Real adversarial cross-checks** (ADJ05) — paused, advisory
  only.
- **Real rulebook injection** (ADJ15/17) — paused.
- **Cross-domain breadth** — clinical / contract benches are
  paused until TSA is stable.

## Methodology

### The matrix

- **8 declarations** mirroring ADJ18, single-item shapes isolating
  one verdict per item:

  | id | text |
  |---|---|
  | `matches` | `1 carry-on bag, matches.` |
  | `large-lithium` | `1 carry-on bag, lithium battery, 200 Wh.` |
  | `large-toothpaste` | `1 carry-on bag, 4 oz toothpaste.` |
  | `pocket-knife` | `1 carry-on bag, 4 inch pocket knife.` |
  | `wine-bottle` | `1 carry-on bag, 1 bottle of wine, 750 ml.` |
  | `small-lithium` | `1 carry-on bag, lithium battery, 50 Wh.` |
  | `small-perfume` | `1 carry-on bag, 3 oz perfume.` |
  | `lighter-disposable` | `1 carry-on bag, disposable lighter.` |

- **5 models** mirroring ADJ12: `gemma4:latest` (8B),
  `llama3.1:8b` (8B), `qwen2.5:3b` (3B), `qwen2.5:1.5b` (1.5B),
  `qwen2.5:0.5b` (0.5B). Different vendors, different family
  scales, all locally-pullable via Ollama.

- Total: **40 cells**.

### Per-cell settings

- `temperature = 0.0`, deterministic.
- `max_retries_per_parent = 3` (the ADJ25 default).
- Per-call timeout: 300 s.
- Per-cell hard cap: 900 s.
- Endpoint: localhost Ollama (`http://127.0.0.1:11434`).

### The driver

A small Rust binary `adj_pr6_bench` (in `adjudication-pipeline`)
reads source / model / endpoint / timeout / retries from env vars,
runs `decompose_hierarchical` against a real Ollama gateway, runs
`check_hierarchical_coverage` on the result, and emits one JSON
record to stdout.

Schema (per cell):

```json
{
  "model": "gemma4:latest",
  "source": "1 carry-on bag, matches.",
  "wallclock_secs": 42.3,
  "total_llm_calls": 7,
  "retry_calls": 1,
  "ir_summary": {
    "node_count": 14,
    "edge_count": 13,
    "kinds_present": ["Document", "Sentence", "Phrase", "Fact", "Entity"]
  },
  "per_level_coverage": {
    "overall_pass": true,
    "flattening_gaps": 0,
    "by_level": [
      { "level": "DocumentToSentence", "passed": true, "gap_count": 0 },
      ...
    ]
  },
  "correlation_completeness": "pass",
  "error": null
}
```

### The harness

A Python script `scripts/adj_pr6_foundation_bench.py` iterates the
matrix, shells out to the binary per cell, captures JSON output,
and writes the aggregated result to
`code/specs/data/adj25-pr6-foundation-bench-YYYY-MM-DD.json`. The
harness persists after every cell so a crash loses at most the
in-flight cell.

## Reproduction

```bash
# Build the bench binary (release for realistic latency).
cargo build -p adjudication-pipeline --bin adj_pr6_bench --release

# Pull the 5 models if not already.
ollama pull gemma4:latest
ollama pull llama3.1:8b
ollama pull qwen2.5:3b
ollama pull qwen2.5:1.5b
ollama pull qwen2.5:0.5b

# Run the full matrix (~30 min – 2 h depending on hardware).
python3 scripts/adj_pr6_foundation_bench.py \
    --endpoint http://127.0.0.1:11434 \
    --binary code/packages/rust/target/release/adj_pr6_bench
```

Subset runs (e.g., for debugging):

```bash
python3 scripts/adj_pr6_foundation_bench.py \
    --models gemma4:latest \
    --declarations matches,large-lithium
```

## Hypotheses

The framework's headline claim — "shrink the model down, push
intelligence into the framework" — is the hypothesis the bench
tests. Specifically:

- **H1 (the big one).** The hierarchical orchestrator + per-level
  retry primitive will drive *some* coverage pass even on the 8B
  reference models. We do not require 100%; we require *any*
  consistent ratio strictly above the empirically-known baseline
  of `~10%` (PR's first-pass typed-quantity recall on the v5 prompt
  in ADJ24).
- **H2 (the small-model claim).** Per-parent fresh-agent retries
  with parent-scoped prompts work for the 1.5B and 0.5B models in
  ways that the prior whole-document retries did not. ADJ24's
  retry loop got llama3.1:8b from 0% → 12% on typed-quantity; the
  hierarchical orchestrator should do meaningfully better than that
  on the same scales because each call is smaller.
- **H3 (the failure-mode claim).** Where the orchestrator fails,
  it fails *loudly* and *structurally* — typed `Primitive`,
  `UnparseableResponse`, or `CoverageUnresolved` error. No silent
  bad output. The audit trail captures the gap that drove the
  failure.

A pass on H3 is necessary for the framework's auditability story
even if H1/H2 only partially hold.

## Known limitations of this bench

1. **No new `decompose-text-vN` prompt yet.** The orchestrator
   relies on whatever prompt `decompose_text` is shipping with
   (currently `v5`, which teaches the v3 flat IR). A model asked to
   decompose a source chunk via this prompt will produce flat-IR
   shaped output; the orchestrator's `parse_child_node` filters
   to the allowed kinds at each level. The hierarchy contract is
   *implicit* — the model is asked for a chunk-of-source
   decomposition without being told "produce a Sentence node".
   Expected consequence: low first-pass pass rate; the retry prompt
   (which IS hierarchy-aware) does most of the lifting. A
   follow-up that lands `decompose-text-v6` should improve
   numbers meaningfully.
2. **The orchestrator's retry budget is fixed at 3 per parent.**
   PR-5 made it `max_retries.min(64)`; the bench uses the default.
3. **Real LLMs only.** The harness does not run against scripted
   clients — those are unit-test territory.
4. **No fact-sheet world knowledge.** ADJ23 surfaced that bag-count
   "1" is a learned-prior-against-typing problem the retry primitive
   doesn't move. ADJ20 (fact sheets) is the structural answer but
   is paused; this bench will likely show the same residual failure
   pattern on the bag-count literal.

## Empirical results

> Bench run: 2026-05-13 on local Ollama, all five models pulled
> (gemma4:latest, llama3.1:8b, qwen2.5:3b/1.5b/0.5b), default
> harness settings (per-cell timeout 180s, max_retries_per_parent
> 3, cell hard cap 600s). Total wallclock for the 40-cell matrix:
> **36.8 minutes**. Raw data:
> [`data/adj25-pr6-foundation-bench-2026-05-13.json`](data/adj25-pr6-foundation-bench-2026-05-13.json).

### Headline numbers

| Metric | Result |
|---|---|
| Cells run | 40 / 40 (no harness errors) |
| Cells producing usable IR (no orchestrator error) | **0 / 40** |
| Cells fully passing per-level coverage | **0 / 40** |
| Cells with correlation completeness | n/a (zero IRs to check) |

The orchestrator failed **every** cell. The framework's reaction was
exactly the shape ADJ26 §"Hypotheses" §H3 predicted: every failure
surfaced as a *typed error* with a structured gap recorded in the
audit trail — no silent bad output anywhere. But H1 and H2 (per-level
coverage actually holds, fresh-agent retries close gaps at small
scales) are empirically refuted by this run.

### Failure-mode breakdown

| Error kind | Cells | What it means |
|---|---|---|
| `CoverageUnresolved(1 gap)` | **36 / 40** | The model produced *something*; the orchestrator accepted it; none of the children matched the level's allowed-kinds filter (`Sentence` / `Discarded` for the Doc→Sentence boundary); the parent ended up with no children; retries hit the budget without producing a Sentence node. |
| `Primitive @ DocumentToSentence` | **4 / 40** | The model's response wasn't parseable JSON at all (the existing `decompose_text` truncation path returned an error after the retry primitive's internal budget exhausted). All 4 are gemma4 cells. |

The 4 gemma4 primitive errors match the ADJ23 finding: gemma4 emits
verbose IR JSON that occasionally truncates mid-string. Bigger
output budget would help but doesn't address the root cause.

### Per-model breakdown

| Model | Cells | IR produced | Fully passing | Avg wallclock | Max wallclock |
|---|---|---|---|---|---|
| gemma4:latest | 8 | 0 | 0 | **124.5s** | 242.9s |
| llama3.1:8b | 8 | 0 | 0 | 64.0s | 74.3s |
| qwen2.5:3b | 8 | 0 | 0 | 43.2s | 57.7s |
| qwen2.5:1.5b | 8 | 0 | 0 | 30.5s | 33.8s |
| qwen2.5:0.5b | 8 | 0 | 0 | 13.7s | 21.5s |

Wallclock scales monotonically with parameter count. The 0.5B model
finishes a cell in ~14s; gemma4 averages 124s with a peak of 4 min.
On the framework's deployment-economics axis this is consistent with
ADJ12 / ADJ17: the small-model tier is fast enough that retries are
affordable.

### Why every cell failed — the system-prompt mismatch

`decompose_hierarchical` dispatches per-parent calls via
`retry_decompose_level`, which in turn invokes `decompose_text`. The
`decompose_text` primitive's *system prompt* is `v5` — written for
the v3 flat IR, instructing the model to produce
`Fact`/`Rule`/`Uncertainty`/`Discarded`/etc. The orchestrator's
*correction prompt* (in the `domain_hint`) asks the model to
"decompose into sentences" / "decompose into phrases".

The model sees two instructions and follows the system prompt,
producing flat IR. The orchestrator's per-level allowed-kinds filter
rejects every `Fact` returned for `Document→Sentence` (only
`Sentence` and `Discarded` are valid there). The Document is left
with zero children. The retry primitive re-prompts; same outcome;
budget exhausts; `CoverageUnresolved` fires.

This was the load-bearing known limitation called out in ADJ26
§"Known limitations" item 1, and ADJ25 PR-6 explicitly deferred a
new prompt:

> "The orchestrator currently relies on whatever prompt
> `decompose_text` is shipping with (currently `v5`, which teaches
> the v3 flat IR). Real-LLM behaviour against a hierarchy-aware
> prompt is measured in PR-6 (the foundation bench)."

The bench has confirmed: no `decompose-text-v6`, no useful coverage.
**The unblock path is to ship `decompose-text-v6`** (or an entirely
new primitive `decompose_level` with its own per-level system
prompt) and re-run.

### Comparison to ADJ23 / ADJ24 baseline

ADJ23 measured typed-quantity recall at 28% first-pass (10% strict
pass) on the same source set. ADJ24's retry loop pushed this to 36%
/ 20%. Those numbers used the v5 prompt's flat-IR contract that *is*
aligned with what the model is being asked for (the typed-quantity
contract is additive on top of flat IR).

ADJ26 measures hierarchical coverage with **0% / 0%**. The gap is
explained entirely by the system-prompt mismatch — the orchestrator's
per-level retry prompt is incompatible with v5's system prompt at
the response-kind level. ADJ23's flat-IR + ADJ22 typed-quantity
contract is the *upper bound* on what the current v5 prompt can
deliver; the hierarchical contract requires a different prompt
entirely.

### What this validates (and doesn't)

**Validated** (per ADJ26 §"Hypotheses" §H3):

- The orchestrator + per-level coverage check + retry primitive
  *machinery* works end-to-end. Every failure is a typed error
  (`CoverageUnresolved` / `Primitive`) the framework can route to a
  retry or escalate. No `Tentative`-style silent-bypass behaviour.
- The error structure is informative enough to direct the next
  intervention without ambiguity: 36/40 cells failed at
  `NoChildrenAtLevel(DocumentToSentence)`, which points at the
  prompt contract, not the orchestrator code.

**Refuted** (per ADJ26 §"Hypotheses" §H1 / §H2 with current `v5`
prompt):

- The hierarchical contract is NOT meetable against `v5`'s flat-IR
  system prompt. No model in the lineup produced even a single
  Sentence node when asked to "decompose into sentences" with
  `v5`'s "produce Fact / Rule / etc." system prompt in force.
- Fresh-agent retries with parent-scoped correction prompts do not
  override a strong system prompt that points elsewhere. This is
  consistent with the broader LLM literature: system-prompt
  contracts dominate user-message corrections.

### Gating condition: NOT met. Cutover stays queued.

Every reasonable threshold from §"Gating condition" fails: strict
(0/40), per-model 50% (0/8 per model), per-level 70% (0% at every
level).

Per the ADJ25 spec, no paused workstream resumes (ADJ14 / 15 / 16 /
17 / 18 / 19 / 20). The cutover itself (PR-7's substantive code work
— retiring `Section`, removing the standalone ADJ22 check, promoting
`CorrelationId` to a struct field) likewise stays queued: removing
the `v5` flat-IR machinery now would leave the framework with no
working decompose path at all.

### Next steps the data justifies

In strict priority order:

1. **Land `decompose-text-v6`.** A new system prompt that teaches
   the LLM the ADJ25 hierarchy taxonomy with worked examples per
   level. This is the single biggest lever — current 0% pass rate
   is bounded above by "model doesn't know what kinds to emit at
   each level". A v6 prompt that names `Sentence` / `Phrase` /
   `Fact` / `TypedComponent` and gives one worked example per is the
   minimum viable change. **Alternatively** add a sibling primitive
   `decompose_level(parent_text, level, gateway)` in `llm-primitives`
   with its own per-level system prompt; the orchestrator switches
   to that primitive for level-boundary calls and `decompose_text`
   stays available for legacy flat-IR consumers.
2. **Re-run this bench against v6.** Same harness, same matrix, same
   threshold candidates. If v6 produces 50%+ cells fully passing
   on the 8B tier, the gate is in reach.
3. **If v6 alone is insufficient at the small-model tier**, the
   ADJ24 learned-prior pattern (small models ignoring instructions)
   probably recurs. The next intervention is constrained-decoding
   on the kind enum (per ADJ23 workstream C). Confirm with bench
   data before committing.

### Notable methodological wins

- **The harness ran the full 40 cells in 36.8 min** — well inside
  the 30 min – 2 h spec estimate. Re-runs against `v6` will fit in
  a coffee break.
- **Per-cell persistence works as designed**: a hypothetical mid-run
  crash would have lost at most the in-flight cell.
- **Failure shapes are typed and consistent** — the data file's
  `error.kind` field cleanly partitioned the 40 cells into 2 groups
  (`CoverageUnresolved` × 36, `Primitive` × 4). This is the
  audit-trail richness ADJ25 was designed for.

## Gating condition for unblocking other workstreams

Per ADJ25's spec, ADJ14 / ADJ15 / ADJ16 / ADJ17 / ADJ18 / ADJ19 /
ADJ20 stay paused until this bench shows reliable per-level
coverage. "Reliable" is not yet operationalised — the empirical
results PR will propose a threshold based on the actual numbers.
Reasonable candidates:

- **Strict**: every cell passes every level (`40/40 cells fully
  green`). Very aggressive; probably unmeetable without
  `decompose-text-v6`.
- **Per-model**: every model passes at least 50% of cells fully.
  Demonstrates the orchestrator works across the size range.
- **Per-level**: every level passes at least 70% across the
  matrix. Identifies the weakest boundary for targeted work.

The PR that lands empirical results picks a threshold and
documents whether it was met. If it isn't, that PR also proposes
which paused workstream to unblock first based on what the bench
revealed (e.g., if ADJ20 fact-sheet handling is the obvious
unblocker for bag-count failures, that becomes the next active
piece).

## See also

- [ADJ25](ADJ25-hierarchical-decomposition.md) — the spec this
  bench validates.
- [ADJ12](ADJ12-small-model-benchmarks.md) — the 5-model lineup.
- [ADJ18](ADJ18-broadened-tsa-empirical-bench.md) — the
  8-declaration set.
- [ADJ23](ADJ23-decomposition-bench.md),
  [ADJ24](ADJ24-typed-quantity-pipeline-wiring.md) — prior
  decomposition benches showing the 10% / 20% ADJ22 baseline this
  bench is designed to beat.

## Status

- v1 — methodology + harness landed
  ([#3107](https://github.com/adhithyan15/coding-adventures/pull/3107),
  merged 2026-05-13).
- v2 — empirical results from the 2026-05-13 Ollama run added
  inline above. Gating condition **NOT met**: 0/40 cells produced
  usable IR under the current `decompose-text-v5` system prompt.
  Next step: land `decompose-text-v6` (or sibling
  `decompose_level` primitive) with hierarchy-aware system prompt
  + worked examples, then re-run this bench.
