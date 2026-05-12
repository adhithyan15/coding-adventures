# ADJ15 — Recursive Rulebook Elicitation: Empirical Results (n=1, 5 models, 2026-05-12)

## Overview

A first measurement of the recursive rulebook-elicitation pattern
([ADJ14](ADJ14-rule-elicitation.md)) on the canonical
[ADJ12](ADJ12-small-model-benchmarks.md) model lineup. The
hypothesis under test:

> If a model has its own elicited rulebook in context at answer
> time, will it stop hallucinating fabricated TSA rules and start
> citing specific rule numbers?

Two arms, same source string, same model: one answer with no
rulebook (the ADJ12 baseline), one answer where the framework first
calls `acquire_rulebook` on the same model to elicit a rulebook
from its weights and injects the resulting text into the answer's
system prompt.

Headline result: **the recursion works at the 3B+ scale and breaks
down at ≤1.5B**, in two distinct failure modes.

## Experimental setup

- **Source string**: `"1 carry-on bag, matches."` (24 bytes, the
  canonical ADJ10 fixture).
- **Models**: gemma4:latest (8B), llama3.1:8b (8B), qwen2.5:3b (3B),
  qwen2.5:1.5b (1.5B), qwen2.5:0.5b (0.5B). Same lineup as ADJ12.
- **Endpoint**: localhost Ollama, `temperature: 0.0`, deterministic.
- **Modes per model**:
  - `none`: Arm A asks the model directly with the v0.7 system
    prompt ("You are a TSA compliance officer..."). No rulebook
    in context.
  - `elicit`: Stage 0 calls
    `adjudication_rulebook::acquire_rulebook` against
    `Role::RuleExtractor` (bound to the same model). The
    resulting rulebook text is injected into Arm A's system
    prompt, with the explicit instruction *"Do not invent any
    additional rules; if a finding is not justified by a specific
    numbered rule below, do not include it"*. Same source string.

- **Counted observations** per (model, mode):
  - Verdict (`COMPLIANT` / `NON-COMPLIANT`).
  - Did the answer cite a specific rule number?
  - Stage 0 elicitation outcome (succeeded / failed; byte count).
  - Wall-clock latency.

Raw run data: [`data/adj15-recursive-rulebook-bench-2026-05-12.json`](data/adj15-recursive-rulebook-bench-2026-05-12.json).

## The table

| Model            | Size | `none` verdict | `elicit` verdict | Rule cited? | Elicit bytes | Flip? |
|------------------|-----:|---------------:|-----------------:|-------------|-------------:|:-----:|
| gemma4:latest    |   8B |     COMPLIANT  |    NON-COMPLIANT | yes ("per rule 5") | 1,938 | ✓ |
| llama3.1:8b      |   8B |     COMPLIANT  |    NON-COMPLIANT | yes ("per rule 2") | 1,709 | ✓ |
| qwen2.5:3b       |   3B |     COMPLIANT  |    NON-COMPLIANT | yes (rules 3 & 9)  | 3,125 | ✓ |
| qwen2.5:1.5b     | 1.5B |     COMPLIANT  |        COMPLIANT | no                 | **FAILED** | ✗ |
| qwen2.5:0.5b     | 0.5B |     COMPLIANT  |        COMPLIANT | no                 | 33,225 | ✗ |

All five models said `COMPLIANT` in `none` mode — every one of them
silently ignored the prohibited-matches issue. ADJ12's
hallucination patterns held: qwen2.5:1.5b returned the
*"matches are allowed in carry-on luggage as long as they are not
lit"* fabrication; qwen2.5:0.5b returned the *"does not exceed the
weight limit of 30 pounds"* fabrication; llama3.1:8b returned the
*"matches do not pose a security risk as long as they are properly
packaged"* fabrication. None of these are real TSA rules.

## What changed at the 3B+ scale

The three larger models flipped their verdict to `NON-COMPLIANT`
and cited specific rule numbers from their own elicited rulebooks.
Concrete answers:

**gemma4:latest** (after eliciting 1,938 bytes of TSA rulebook):
> "Matches are classified as flammable materials, which are strictly
> prohibited in all carry-on and checked baggage **per rule 5**.
> Therefore, the passenger is carrying an item that violates TSA
> regulations. VERDICT: NON-COMPLIANT"

**llama3.1:8b** (after eliciting 1,709 bytes):
> "The passenger is carrying matches in their carry-on bag.
> According to **rule 2**, prohibited items include but are not
> limited to flammable liquids and solids, which includes matches.
> Therefore, the passenger's declaration indicates a non-compliance
> with the TSA rules per rule 2."

**qwen2.5:3b** (after eliciting 3,125 bytes):
> "RULES INVOLVED: 3. Passengers must declare any liquids, gels,
> aerosols, and creams … 9. Passengers are required to declare any
> items that may interfere with security screening equipment …
> Therefore, the passenger is non-compliant."

The three citation patterns are notably *imperfect* — they all
correctly identify the matches issue but route through somewhat
shaky categorical reasoning (matches as "flammable materials";
matches as "flammable solids"; matches as "liquids/gels because
flammable"). A human reviewer would flag these for clarification.
The framework's defense against this is the `Tentative` trust
tier: every one of these rulebooks ships with `validation_passed
= false` (the rulebook IR didn't satisfy ADJ01 v3 — see *Caveats*
below) and must be promoted to `Reviewed` by a domain expert
before production use.

But the *verdict* is correct in all three cases, with rule
citations that make the reasoning auditable, where the same model
without a rulebook simply confabulated.

## What broke at the ≤1.5B scale

Two distinct failure modes, not just "smaller model = worse output".

**qwen2.5:1.5b — elicitation primitive itself fails.**
`acquire_rulebook` could not produce a usable rulebook. The
elicit_rules call returned, but `decompose_text` against the
resulting text errored out before producing IR (the
`stage0_log` shows no "elicited N bytes" completion line — only
the start-of-elicit line). The demo falls back to no-rulebook
mode, and the answer is byte-for-byte identical to the `none`
baseline. The framework's behaviour here is correct (graceful
degradation, audit-trail records the failure), but the recursive
hypothesis is untested at this scale because the recursion
never engaged.

**qwen2.5:0.5b — elicitation succeeds but is unusable.** The
smallest model dumped 33,225 bytes of rulebook-like prose (over
16× the size of gemma4's elicitation, and at the boundary of
the 8,192-token output cap). The Stage 0 succeeded in the
"primitive returned text" sense; whether the *content* of those
33 KB resembles a TSA rulebook is highly suspect (its
single-source `none`-mode answer fabricated a "30-pound weight
limit" rule, suggesting the model has no coherent internal
representation of TSA rules to elicit). When the elicited
"rulebook" is injected, the answer doesn't change in any way that
matters: same `COMPLIANT` verdict, no rule citations, just a
slightly different weight-limit fabrication (`50 kg` instead of
`30 pounds`). The recursion ran end-to-end but produced no signal.

The two failure modes are both informative. At 1.5B, the
elicitation pipeline itself has a load-bearing failure that
needs investigation. At 0.5B, the elicitation runs but produces
output the model can't *use* — the rulebook ends up as noise in
the prompt rather than as authoritative rules.

## Caveats

1. **n = 1 test case**. One source string ("1 carry-on bag,
   matches.") proves only that the recursive pattern can flip a
   verdict on this particular adjudication. Generalisation
   requires running across the ADJ10 worked example's other
   declaration shapes (toothpaste / perfume / lithium battery /
   wine / pocket knife / lighter / strike-anywhere matches) and
   counting flip rates across an evaluation set.

2. **`validation_passed = false` for every elicit run.** All five
   elicitations produced JSON IR that did not pass
   `adjudication_ir::validate` — none of them satisfied the v3
   graph-IR rules. The framework still surfaces the rulebook (the
   `Tentative` tier permits this), but the audit trail records
   the gap. The fix path is to land
   [`decompose_text` v4](https://github.com/adhithyan15/coding-adventures/blob/main/code/packages/rust/llm-primitives/src/decompose_text.rs)
   prompt fully (it's on main as of #3026) and re-run; v4 teaches
   the graph shape with worked examples, so future elicit runs
   should produce well-formed IR more often. This run captured
   the state of the world at 2026-05-12 ~15:55 PT — v4 prompt was
   on main but the elicitation primitive was still
   `decompose-text-v3`-based for some configurations.

3. **Circular hallucination risk is not addressed by this
   measurement.** All three flips on the 3B+ tier used citations
   from the model's *own* elicited rulebook. A model can elicit a
   plausible-sounding but wrong rule (e.g., gemma4's "rule 5:
   flammable materials are prohibited" is directionally correct
   but the rule's categorical leap to "matches are flammable
   materials" is the model's own inference, not a recognised TSA
   classification). The verdict flips for the right reason in
   spirit, but the audit trail surfaces the categorical leap for
   review. Multi-model adversarial elicitation
   ([ADJ14 Open Question §1, §3](ADJ14-rule-elicitation.md))
   reduces this risk by requiring agreement across model
   families before promoting a rule's effective trust.

4. **Adversarial elicitation not measured here.** This run uses
   the simplest possible Stage 0: one model elicits, the same
   model answers. A follow-up that elicits from gemma4 and
   llama3.1 independently and uses the agreement set as the
   injected rulebook would be a much stronger test.

5. **The 1.5B failure mode is a bug, not a result.** The
   elicitation primitive should fail gracefully with a typed
   error, not silently drop the rulebook. Investigation needed.

## What this tells us about the framework

The recursive pattern ("framework applied to itself") **works at
the scales where it has the most engineering value** — the 3B and
8B class models that are deployable in regulated / air-gapped /
on-device environments, where frontier-API access isn't an option.
At those scales, the framework converts the "model invents
fabricated rules" failure mode (ADJ12's headline finding) into
"model cites elicited rules whose categorical leaps a human can
review". Throughput goes up; auditability goes up; the failure
mode shifts from "silent hallucination at answer time" to "loud
categorical leap recorded in the audit trail" — which is the
right direction.

At ≤1.5B, the recursion doesn't help yet. Two follow-ups would
clarify whether this is a fundamental small-model limit or a
fixable engineering issue:

1. Investigate qwen2.5:1.5b's elicitation failure — the framework
   should produce a typed error rather than silently degrading.
2. Adversarial elicitation across gemma4 + llama3.1, with
   `qwen2.5:3b` answering against the *agreement set*. If a 3B
   model can use a curated cross-model rulebook well, the
   small-model failure mode might be elicitation-only, not
   reasoning-only.

## Status

Single-shot empirical measurement, n=1 test case, 2026-05-12. The
result patterns are striking enough to publish, but they are not
a benchmark in the strict sense — that requires an evaluation set
and a ground-truth verdict per case. A follow-up that runs across
the ADJ10 declaration variants on the same 5-model lineup is the
natural next step.

## Reproduction

```bash
# Build the demo:
cargo build -p adjudication-tsa-demo --release

# Run a single model in elicit mode (replace gemma4:latest as needed):
ADJ_DEMO_MODEL=gemma4:latest \
  ADJ_DEMO_ENDPOINT=http://127.0.0.1:11434 \
  ADJ_DEMO_SOURCE="1 carry-on bag, matches." \
  ADJ_DEMO_IR_MODE=hand \
  ADJ_DEMO_RULEBOOK_MODE=elicit \
  ADJ_DEMO_TIMEOUT_SECS=300 \
  ./target/release/adjudication-tsa-demo
```

The full benchmark is reproducible by iterating over the 5 models
and the two modes (`none`, `elicit`); the canonical script is at
[`data/adj15-recursive-rulebook-bench-2026-05-12.json`](data/adj15-recursive-rulebook-bench-2026-05-12.json)
(includes wall-clock latencies and raw Arm A answers for every
configuration).
