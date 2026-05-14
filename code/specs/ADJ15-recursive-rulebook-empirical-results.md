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

## Addendum (2026-05-12): per-model elicited rulebooks captured

A follow-up run with `ADJ_DEMO_DUMP_RULEBOOK=1` (landed in
`adjudication-tsa-demo` v0.9) captured the actual rulebook text
each model produced during elicitation. Raw data:
[`data/adj15-rulebook-contents-2026-05-12.json`](data/adj15-rulebook-contents-2026-05-12.json).
The headline observation: **the three 3B+ rulebooks differ
substantially in citation quality, categorical reasoning, and
calibration.** Each model's elicited rules below, plus the rule
it cited in its `elicit`-mode answer:

### gemma4:latest (1,938 bytes, 8 rules)

> COVERAGE: TSA carry-on rules are based on general guidelines and
> historical regulations; specific, real-time operational changes
> (e.g., post-pandemic adjustments, temporary mandates) may be
> missing.
>
> 1. All liquids, aerosols, gels, creams, and pastes carried in
>    carry-on baggage must be in containers no larger than 3.4
>    ounces (100 milliliters) each, provided that these containers
>    are placed within a single, clear, quart-sized plastic bag,
>    and only one such bag is permitted per passenger.
> 2. Exceptions to the liquid rule include personal medications …
> 3. All lithium batteries … must be carried in carry-on baggage …
> 4. Lithium batteries with an energy capacity exceeding 100 Wh …
> **5. Items deemed weapons, explosives, flammable materials, or
>    illegal substances are strictly prohibited in all carry-on and
>    checked baggage, including, but not limited to, firearms,
>    knives (with certain exceptions for personal grooming tools),
>    fireworks, and chemical agents.**
> 6. Electronics containing lithium batteries …
> 7. Passengers must declare any items that are restricted …
> 8. While the TSA reserves the right to screen any item …

**Cited at answer time**: *"Matches are classified as flammable
materials, which are strictly prohibited in all carry-on and
checked baggage per rule 5."*

Rule 5 is real-TSA-shaped: weapons / explosives / flammable
materials / illegal substances. The model made a **categorical leap
at answer time**: matches → "flammable materials" → rule 5 fires.
The leap is plausible (matches are flammable) but the rule itself
does not say "matches are flammable materials"; that classification
is the model's own inference, made inside the forward pass that
produced the answer, untraceable beyond the answer text. A
reviewer reading the rulebook can see this and either accept the
categorical leap or flag it.

### llama3.1:8b (1,706 bytes, 10 rules)

> COVERAGE: TSA carry-on rules as of ~2024; post-2024 amendments
> may be missing.
>
> 1. Liquids, gels, and aerosols in containers of 3.4 ounces (100
>    milliliters) or less are allowed in a clear, quart-sized
>    zip-top bag per the Transportation Security Administration's
>    (TSA) 3-1-1 liquids rule.
> **2. Prohibited items include but are not limited to: firearms,
>    ammunition, explosives, flammable liquids and solids, poisons,
>    and certain chemicals per 49 CFR § 1540.111(a).**
> 3. Electronic devices … must be removed from carry-on bags …
> 4. Sharp objects like scissors, knives, and razors are allowed in
>    checked baggage but prohibited in carry-on bags except when …
> 5. Tools such as pliers, screwdrivers, and wrenches are permitted
>    in carry-on bags if they fit in a clear, quart-sized zip-top
>    bag.
> 6. Fireworks, sparklers, and other pyrotechnic devices are
>    prohibited in both checked and carry-on baggage per 49 CFR §
>    1540.111(a).
> 7. Certain medical equipment …
> 8. Lithium batteries must be protected from accidental activation
>    … per IATA's Dangerous Goods Regulations.
> 9. Certain items like firearms, ammunition, and explosives
>    require special handling and documentation …
> 10. The TSA reserves the right to inspect any item …

**Cited at answer time**: *"According to rule 2, prohibited items
include but are not limited to flammable liquids and solids, which
includes matches."*

Strikingly, **llama3.1 cited a real CFR provision** — 49 CFR §
1540.111(a) — which is genuinely the section governing prohibited
items in carry-on baggage (and groups matches under flammable
solids in the hazmat classification, UN3091/UN3092). This is the
most defensible of the three citations: the rule it cites is real,
the cited authority is real, and matches really are classified as
flammable solids under hazmat regulations. The categorical leap is
still happening (no rule explicitly mentions matches), but the
rule's wording covers the inference more cleanly than gemma4's.

Note: rule 4's "doctor's note exception" for sharp objects is
**fabricated** — TSA has no such exception. The model fabricated a
believable-but-wrong rule alongside the believable-and-correct
ones. A reviewer would flag it.

### qwen2.5:3b (3,124 bytes, 10 rules)

> COVERAGE: This list covers TSA … guidelines specifically related
> to carry-on baggage as of my last update in 2024, but post-2024
> changes and specific airline policies may not be included.
>
> 1. Passengers are required to declare all items they intend to
>    bring onto the aircraft during the security screening process.
>    `UNCERTAIN: Specific wording on declaration requirements …
>    varies by TSA directive, which is not always publicly
>    available in a single document.`
> 2. Electronic devices with lithium-ion batteries … if they exceed
>    100 Wh … `UNCERTAIN: …`
> **3. Passengers must declare any liquids, gels, aerosols, and
>    creams that exceed 3.4 ounces (100 milliliters) in a single
>    container. `UNCERTAIN: …`**
> 4. Passengers must declare any sharp objects such as knives and
>    scissors that exceed 3 inches (7.62 cm) in length. `UNCERTAIN`
> 5. Passengers are required to declare any weapons …
> 6. Passengers must declare any firearms …
> 7. Passengers are required to declare any items considered
>    dangerous goods …
> 8. Passengers must declare any items that exceed the size limits …
> **9. Passengers are required to declare any items that may
>    interfere with security screening equipment, such as large
>    electronic devices or bulky clothing. `UNCERTAIN: …`**
> 10. Passengers must declare any items that exceed the weight
>    limits for carry-on baggage as specified by TSA regulations.

**Cited at answer time**: *"matches, which fall under the category
of liquids/gels/aerosols/creams (rule 3) and could potentially
interfere with security screening equipment (rule 9) … the passenger
is non-compliant."*

Two notable patterns:

1. **qwen2.5:3b correctly self-flags uncertainty** on its own
   rulebook items — every single rule has an `UNCERTAIN:` tag the
   model attached itself. The model knows it doesn't know which
   precise wording is authoritative.

2. **It does NOT propagate that uncertainty to its answer.** The
   answer confidently cites "rule 3" and "rule 9" — both of which
   are tagged `UNCERTAIN` in the model's own rulebook — without
   surfacing that uncertainty. The verdict is right
   (NON-COMPLIANT) but the reasoning is wrong (matches are not
   liquids/gels/aerosols/creams; the screening-interference
   argument is a stretch).

This is an interesting **calibration failure pattern**: the model is
honest about its uncertainty in elicit mode and then forgets it at
answer time. A pipeline-level fix would propagate the `UNCERTAIN:`
tags from the rulebook into a probability weight on each rule
(directly applicable to the ProbLog work in
[ADJ16](ADJ16-engine-programmatic-adjudication.md)) so that
"compliance" verdicts derived from uncertain rules carry a lower
marginal probability.

### Patterns across all three

1. **Citations are imperfect across the board.** All three flips
   happen through a categorical leap that's auditable in the
   rulebook but happens *inside the forward pass* at answer time.
   ADJ16's engine-programmatic adjudication addresses this
   directly: the engine's proof DAG would show which rule unified
   with which fact, making the leap an explicit step that can be
   reviewed and rejected.

2. **Rulebook quality scales with model size**, but not
   monotonically with rulebook length. llama3.1's 1,706-byte
   rulebook has real CFR citations; qwen2.5:3b's 3,124-byte rulebook
   has more rules but worse rules. Size of elicited text is not
   a quality signal.

3. **Calibration is decoupled from rulebook accuracy.** qwen2.5:3b
   is well-calibrated about its rulebook (UNCERTAIN tags) but
   poorly calibrated about its answer (cites UNCERTAIN rules
   confidently). gemma4 has neither calibration signal in either
   direction. The framework would benefit from propagating
   rulebook-level uncertainty into answer-level probability —
   which is exactly the ProbLog extension ADJ16 specifies.

4. **All three rulebooks have at least one fabrication.**
   - gemma4's rule 5 doesn't explicitly cover matches; the
     classification is the model's leap.
   - llama3.1's rule 4 fabricates a "doctor's note exception".
   - qwen2.5:3b's rule 4 has wrong dimensions (3" vs the actual
     2.36" / 60mm TSA limit).
   Human review remains necessary; the `Tentative` trust tier is
   doing its job by refusing to promote any of these without
   sign-off.
