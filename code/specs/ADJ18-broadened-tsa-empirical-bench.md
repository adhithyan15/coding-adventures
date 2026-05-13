# ADJ18 — Broadened TSA Empirical Bench: methodology + harness

## Overview

ADJ12, ADJ15, and ADJ17 measured the framework's Arm A behaviour on
**a single source string** (`"1 carry-on bag, matches."`) across 5
models and 3 rulebook-injection modes. The result patterns were
striking enough to publish — small models flip from hallucinated
COMPLIANT to defensible NON-COMPLIANT with rule citations once an
adversarial rulebook is in context — but n=1 doesn't generalise.

ADJ18 broadens the bench to **8 single-item declarations** designed
to isolate one verdict per item (3 expected COMPLIANT, 5 expected
NON-COMPLIANT), still on the same 5-model lineup, and adds the
**v0.12 priming dispatch** as a third Arm A mode so we can measure
whether the two-turn protocol from PR #3057 reduces truncation in
practice.

Total matrix: **8 declarations × 5 models × 3 Arm A modes = 120
cells**. Each cell is one (or two, in priming mode) Ollama call.

This spec is methodology-only — the actual results land as a
follow-up data file (`code/specs/data/adj18-tsa-bench-YYYY-MM-DD.json`)
and an addendum to this spec once the bench has been run end-to-end.

## What we're measuring

Per cell, we capture:

1. **Verdict** — `COMPLIANT` / `NON-COMPLIANT` / `null`
   (parse-failed). Compared against an expected verdict per
   declaration; flip rate is the primary metric.
2. **Truncation flag** — `Arm A failed: output truncated`
   surfaces with `finish_reason: Stop(MaxTokens)`. The v0.12
   priming mode is specifically designed to reduce this; ADJ18
   measures the delta.
3. **Latency** — wall-clock per call. Priming mode pays for one
   extra round-trip (turn 1 ACK); the bench captures whether the
   reduction in retry rate makes up for it.
4. **Token usage** — input/output tokens. Priming mode summed
   across both turns. Useful for cost accounting.
5. **Raw Arm A block** — the full text the model produced (capped
   at 4 KB per cell). Required for spot-checking reasoning quality
   and noting hallucination patterns.

What we are *not* measuring in this bench:

- **Arm B (full pipeline)** — Arm B's verdict depends on the IR
  extractor's behaviour, which is a separate variable. ADJ18 keeps
  the IR in `HandBuilt` mode so the bench isolates the
  rulebook-injection effect on Arm A. Arm B bench is queued as a
  follow-up.
- **Arm C (engine arm)** — Arm C is deterministic given a
  rulebook, so per-cell variance is zero. Arm C measurement
  requires a richer source IR than `HandBuilt` produces today; the
  fact-elicitation primitive in ADJ19 (planned) is the unblocker.
- **Adversarial elicit mode** — ADJ17 already covered the
  adversarial elicit path on the matches declaration. Re-running
  it across all 8 declarations is the natural ADJ18.5 follow-up
  but doubles bench wallclock for what's likely a small marginal
  signal beyond the ADJ17 findings. We'll add it after the
  fixture-rulebook baseline is in.

## The declaration set

Eight single-item declarations, each isolating one
prohibited-or-permitted decision. The text is intentionally
minimal — one bag and one item — so the model can't pivot on
some other declared item to flip the verdict.

| ID | Declaration | Expected | Rationale |
|---|---|---|---|
| `matches` | `"1 carry-on bag, matches."` | NON-COMPLIANT | Strike-anywhere matches prohibited under TSA flammable rule. |
| `large-lithium` | `"1 carry-on bag, lithium battery, 200 Wh."` | NON-COMPLIANT | Lithium batteries above 100 Wh prohibited in carry-on. |
| `large-toothpaste` | `"1 carry-on bag, 4 oz toothpaste."` | NON-COMPLIANT | 4 oz exceeds the 3.4 oz / 100 ml liquid limit. |
| `pocket-knife` | `"1 carry-on bag, 4 inch pocket knife."` | NON-COMPLIANT | Pocket knife blade > 2.36 in (60 mm) prohibited in carry-on. |
| `wine-bottle` | `"1 carry-on bag, 1 bottle of wine, 750 ml."` | NON-COMPLIANT | 750 ml liquid exceeds the 3.4 oz / 100 ml limit. |
| `small-lithium` | `"1 carry-on bag, lithium battery, 50 Wh."` | COMPLIANT | Lithium batteries under 100 Wh permitted in carry-on. |
| `small-perfume` | `"1 carry-on bag, 3 oz perfume."` | COMPLIANT | 3 oz fits within the 3.4 oz liquid limit. |
| `lighter-disposable` | `"1 carry-on bag, disposable lighter."` | COMPLIANT | One disposable lighter per passenger permitted. |

The "expected" column is what the **TSA's actual published rules
say**, not what any particular model thinks. A correct verdict
matches the expected column; the bench measures the deviation
between model output and the published rules.

## The 5-model lineup

Unchanged from ADJ12 / ADJ15 / ADJ17 — gemma4:latest (8B),
llama3.1:8b (8B), qwen2.5:3b (3B), qwen2.5:1.5b (1.5B),
qwen2.5:0.5b (0.5B). Different vendors, different family scales,
all Ollama-pullable. The point is small-model behaviour:
gemma4/llama3.1 are reference 8B baselines; qwen2.5 down to 0.5B
is the small-deployment story.

## The 3 Arm A modes

1. **`none`** — no rulebook injected. Arm A receives only the
   demo's default v0.12 system prompt
   (`build_raw_system_prompt(None)`) and the declaration text.
   The model relies on whatever ghost of TSA rules its training
   data contains. This is the **ADJ12 hallucination baseline**.
2. **`fixture-single`** — `ADJ_DEMO_RULEBOOK_MODE=fixture` injects
   the hand-authored canonical TSA rulebook
   (`fixture_tsa_rulebook()`) into the Arm A system prompt;
   `ADJ_DEMO_ARM_A_MODE=single-turn` keeps the v0.11 dispatch.
   This is the **single-turn rulebook-injection baseline**.
3. **`fixture-priming`** — same fixture rulebook, but
   `ADJ_DEMO_ARM_A_MODE=priming` engages the v0.12 two-turn
   dispatch. Turn 1 hands the model the rulebook with an
   ACK-only instruction; turn 2 sends the declaration and demands
   a verdict-first answer. This is the **truncation-hardened
   variant** of mode 2.

The matrix is intentionally structured so mode 2 vs mode 3 isolates
the priming effect (same rulebook, different dispatch), and mode 1
vs mode 2 isolates the rulebook-injection effect (same dispatch,
different rulebook).

## Hypotheses being tested

H1. **Rulebook injection improves verdict accuracy across the
    declaration set.** Mode 2 should outperform mode 1 on flip
    rate against the expected verdict. ADJ15 showed this on the
    matches declaration; ADJ18 tests if it generalises.

H2. **Priming reduces truncation on verbose models.** Mode 3
    should show a lower truncation rate than mode 2 specifically
    for gemma4 (the model that hit the 512-token cap in ADJ17
    against the adversarial rulebook). The mechanism: turn 1
    consumes the rulebook silently, so turn 2's output budget is
    spent on the verdict, not on rulebook narration.

H3. **Priming preserves or improves verdict accuracy.** If
    priming reduces truncation without changing the model's
    reasoning, mode 3's verdict accuracy should be ≥ mode 2's.
    Failure mode to watch: the model treats turn 1 as the question
    and produces a verdict in the ACK step, then ignores or
    confuses the turn 2 declaration. We'll spot-check raw answers
    for this.

H4. **The mode-1 hallucination pattern is consistent across
    declarations.** All 5 models should default to a "this is
    fine, here's a fabricated rule" answer on the matches
    declaration (per ADJ12). ADJ18 tests whether this pattern
    holds on the other 7 declarations or whether some items
    (lithium, pocket knife) are robust to it through training-data
    coverage.

## Harness

The harness is a Python script at
[`scripts/adj18_bench.py`](../../scripts/adj18_bench.py). It:

- Iterates the 8 × 5 × 3 matrix, setting env vars per cell.
- Calls the built `adjudication-tsa-demo` binary per cell as a
  subprocess.
- Parses the Arm A stdout block via regex (verdict, latency,
  tokens, truncation flag).
- Writes a JSON file with one record per cell, persisted after
  every cell so a crash loses at most the cell in flight.
- Supports `--resume` so an overnight bench can be restarted.
- Accepts subset filters (`--models`, `--modes`, `--declarations`)
  for testing.

The harness uses the same conventions as the existing manual
benches (ADJ15/ADJ17 JSON data files) so the analysis tooling can
reuse parsing logic.

## Reproduction

```bash
# Build the demo binary first:
cargo build -p adjudication-tsa-demo --release

# Run the bench (allow 2-4 hours):
python3 scripts/adj18_bench.py \
    --endpoint http://127.0.0.1:11434 \
    --cache-dir /tmp/adj18_cache \
    --out code/specs/data/adj18-tsa-bench-$(date +%F).json

# Resume an interrupted run:
python3 scripts/adj18_bench.py \
    --resume \
    --out code/specs/data/adj18-tsa-bench-2026-05-13.json
```

The full bench is roughly 2-4 hours on commodity hardware against a
local Ollama. With the v0.10.1 cache fix, repeat runs against the
same `--cache-dir` replay from disk in seconds; the first cold run
is the slow one.

## What the bench data file should look like

```json
{
  "harness_version": "adj18-v1",
  "endpoint": "http://127.0.0.1:11434",
  "binary": "code/packages/rust/target/release/adjudication-tsa-demo",
  "cells": [
    {
      "cell_id": "matches::gemma4:latest::none",
      "declaration_id": "matches",
      "declaration_text": "1 carry-on bag, matches.",
      "expected_verdict": "NON-COMPLIANT",
      "model": "gemma4:latest",
      "mode": "none",
      "rationale": "Strike-anywhere matches prohibited under TSA flammable rule.",
      "result": {
        "verdict": "COMPLIANT",
        "finish_reason": "Stop",
        "latency_ms": 5664,
        "input_tokens": 98,
        "output_tokens": 308,
        "truncated": false,
        "wallclock_s": 25.6,
        "exit_code": 0,
        "raw_block": "...",
        "stderr_excerpt": ""
      }
    },
    ...
  ]
}
```

## Status

Methodology and harness landed. Data collection is a follow-up
that runs against a live Ollama instance. After the bench
completes, the data file goes into `code/specs/data/` and this
spec gets an "Empirical results" section appended summarising the
flip rates and truncation deltas by model and mode.

## Follow-ups

- **ADJ18 results addendum** — populate this spec with the
  empirical findings once the bench has been run.
- **ADJ18.5 adversarial follow-up** — add the
  `adversarial:gemma4:latest,llama3.1:8b` mode as a fourth mode
  across all 8 declarations. Tests whether the ADJ17 flip
  pattern generalises beyond matches.
- **Cross-domain bench** — same harness shape against
  clinical-demo and contract-demo. Tests whether the
  rulebook-injection pattern is TSA-specific or generalises to
  other rule-based-decision domains. Will land as a separate spec.
- **Fact-elicitation bench** — once ADJ19 (fact sheets) lands,
  add Arm C measurements per cell. This is when we get a real
  apples-to-apples LLM-vs-engine comparison.
