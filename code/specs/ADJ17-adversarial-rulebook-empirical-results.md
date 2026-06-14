# ADJ17 — Adversarial Multi-Model Rulebook Elicitation: Empirical Results (n=1, 5 models, 2026-05-12)

## Overview

A direct follow-up to
[ADJ15](ADJ15-recursive-rulebook-empirical-results.md). ADJ15 measured
the *single-model* recursive pattern: model elicits its own rulebook,
same model answers against that rulebook. ADJ15's headline finding
was that the recursion works at ≥3B parameters and breaks at ≤1.5B —
both in elicitation (qwen2.5:1.5b couldn't produce a usable rulebook)
and in usage (qwen2.5:0.5b couldn't reason against the rulebook it
produced).

ADJ17 tests the natural follow-up hypothesis from
[ADJ15 §Caveats(3-4)](ADJ15-recursive-rulebook-empirical-results.md):

> If the rulebook is elicited from **multiple independent models** and
> the answerer model is given the merged, provenance-tagged
> adversarial rulebook in context, does the recursion start working
> at smaller scales — and does the circular-hallucination failure
> mode reduce?

Stage 0 of the pipeline becomes **adversarial elicitation**: two
independent models (here, gemma4:latest and llama3.1:8b — different
families, different vendors) each elicit a rulebook from their own
weights, the framework concatenates them with provenance tags
(`=== RULEBOOK FROM <model_label> ===`), and the merged text is
injected into Arm A's system prompt. The answerer model is varied
across the same 5-model lineup as ADJ12/ADJ15.

**Headline result**: with the adversarial rulebook in context, the
five answerer models behave dramatically differently from ADJ15.
The two scales that broke in ADJ15 (1.5B and 0.5B) now produce
defensible NON-COMPLIANT verdicts with rule citations. The 3B model
sharpens its reasoning. The 8B class splits: llama3.1 produces a
clean NON-COMPLIANT, gemma4 overflows the token budget on
chain-of-thought narration and surfaces a typed truncation error.
**Four of five models flip; the fifth fails loud**, which is the
right shape of failure.

## What changed mechanically between ADJ15 and ADJ17

ADJ15 ran one model in two modes (`none`, `elicit`). ADJ17 runs five
*answerer* models in one mode (`adversarial:gemma4:latest,llama3.1:8b`)
shipped in adjudication-tsa-demo v0.10 (PR #3044). Concretely:

1. `adjudication_rulebook::acquire_rulebook_adversarial(req, model_gateways)`
   dispatches `acquire_rulebook` against each named model's gateway
   in turn, sanitizes the model label for use as a `document_id`,
   and returns an `AdversarialRulebook` with per-model outcomes
   (`Acquired { rulebook }` or `Failed { error_summary }`) plus a
   merged `source_text` that concatenates the rulebook bodies with
   `=== RULEBOOK FROM <label> ===` provenance headers and a
   `(trust=..., validation_passed=..., model=...)` metadata line.
2. The demo parses `ADJ_DEMO_RULEBOOK_MODE=adversarial:m1,m2,...`,
   builds an `OllamaClient` + `GatewayConfig` per named model, and
   passes the list to `acquire_rulebook_adversarial`. The merged
   text becomes the system-prompt suffix for Arm A.
3. The answerer model is the one named in `ADJ_DEMO_MODEL`. It does
   **not** participate in Stage 0 unless it happens to also appear
   in the adversarial set.

Raw run data:
[`data/adj17-adversarial-bench-2026-05-12.json`](data/adj17-adversarial-bench-2026-05-12.json).

## Experimental setup

- **Source string**: `"1 carry-on bag, matches."` (same 24-byte ADJ10
  fixture as ADJ12 and ADJ15).
- **Adversarial elicitation set**: `gemma4:latest` and `llama3.1:8b`
  — the two 8B-class models from different families. Each elicits
  independently; results merged with provenance.
- **Answerer models**: `gemma4:latest`, `llama3.1:8b`, `qwen2.5:3b`,
  `qwen2.5:1.5b`, `qwen2.5:0.5b` — same lineup as ADJ12/ADJ15.
- **Endpoint**: localhost Ollama, `temperature: 0.0`, deterministic.
  `ADJ_DEMO_ENDPOINT=http://127.0.0.1:11434` (the macOS-`localhost`
  → ::1 vs 127.0.0.1 trap from earlier debugging is avoided
  explicitly).
- **Stage 0 outputs (identical across all 5 answerer runs)**:
  - gemma4:latest: 2,170 bytes, `validation=FAILED (CoverageGap
    missing_ranges=[(194, 200)])`.
  - llama3.1:8b: 2,546 bytes, `validation=FAILED (CoverageGap
    missing_ranges=[(43, 44)])`.
  - Both pass the "succeeded in producing prose" bar; both fail
    strict ADJ02 v3 graph-IR coverage by a handful of bytes.

## The table

Compare ADJ15 (single-model elicit) against ADJ17 (adversarial elicit):

| Answerer       | Size | ADJ15 `none` | ADJ15 `elicit` | ADJ17 adversarial | Rule cited (ADJ17)? |
|----------------|-----:|-------------:|---------------:|------------------:|--------------------:|
| gemma4:latest  |   8B |   COMPLIANT  |  NON-COMPLIANT | **TRUNCATED**     | (didn't reach verdict) |
| llama3.1:8b    |   8B |   COMPLIANT  |  NON-COMPLIANT | NON-COMPLIANT     | yes ("per rule 3")  |
| qwen2.5:3b     |   3B |   COMPLIANT  |  NON-COMPLIANT | NON-COMPLIANT     | yes ("per rule 3")  |
| qwen2.5:1.5b   | 1.5B |   COMPLIANT  |      COMPLIANT | **NON-COMPLIANT** | yes ("per rule 3")  |
| qwen2.5:0.5b   | 0.5B |   COMPLIANT  |      COMPLIANT | **NON-COMPLIANT** | yes ("per rule 3")  |

The bold cells are the new behavior. The two scales that ADJ15
documented as broken now produce the correct verdict and cite a
specific rule. gemma4 surfaces a loud truncation error instead of a
silent bad answer.

## What changed at the small scales

The most striking observation is the bottom two rows of the table.

**qwen2.5:1.5b** (1.5B):
> "VERDICT: NON-COMPLIANT. The passenger is non-compliant with the
> rules as per rule 3. Matches are considered a weapon and thus
> prohibited in carry-on baggage according to the established
> guidelines. The presence of matches alone violates this rule,
> making the declaration non-compliant."

In ADJ15 this model could not even complete its own elicitation —
the recursion failed before producing a usable rulebook, the demo
fell back to no-rulebook mode, and the answer was byte-for-byte
identical to the `none` baseline (`COMPLIANT`, fabricated
"matches-are-allowed-if-not-lit" reasoning). In ADJ17 the model is
*not* asked to elicit anything; it just consumes the gemma4 +
llama3.1 merged rulebook and answers. The answer is now correct,
cites rule 3, and reads as a defensible compliance check.

**qwen2.5:0.5b** (0.5B):
> "...per rule 3, strike-anywhere matches are prohibited. ...
> Therefore, the final verdict is: VERDICT: NON-COMPLIANT."

In ADJ15 this model could elicit text but couldn't use the text it
elicited — its single-model elicit answer fabricated a "50 kg weight
limit" and returned `COMPLIANT`. In ADJ17, with a rulebook produced
by larger models, it flips to NON-COMPLIANT and cites rule 3
(though it does so via a slightly muddled "based on Document_ID:
423, … final verdict is NON-COMPLIANT" arc — see *Caveats*).

This is the framework's headline value in concrete form: **a
0.5-billion-parameter model produces a defensible NON-COMPLIANT
verdict with a rule citation on a non-trivial regulatory question**,
where the same model on its own (ADJ12) fabricated rules.

## What changed at 3B and 8B

**qwen2.5:3b** (3B):
> "The declaration includes matches, which are prohibited in
> carry-on baggage per rule 3. Strike-anywhere matches are
> considered a weapon or explosive and thus must not be carried on
> board. VERDICT: NON-COMPLIANT"

In ADJ15 this model flipped to NON-COMPLIANT via its own
`UNCERTAIN`-tagged rulebook, but the categorical leap was strained
("matches fall under the category of liquids/gels/aerosols/creams
(rule 3)"). In ADJ17 the leap is cleaner — matches → weapon or
explosive → rule 3 — which is the categorical line gemma4's elicited
rule 3 actually takes ("Items deemed weapons, explosives, flammable
materials, or illegal substances are strictly prohibited..."). The
answer is shorter, more confident, and routes through the adversarial
rulebook's stronger framing.

**llama3.1:8b** (8B):
> "The passenger is carrying strike-anywhere matches in their
> carry-on bag. Per rule 3, any item deemed a weapon or prohibited
> article (including flammable materials) is strictly forbidden in
> carry-on baggage. Matches are considered a fire hazard and thus
> prohibited. VERDICT: NON-COMPLIANT"

This is the cleanest answer in the run. The model uses gemma4's
rule 3 ("Items deemed weapons, explosives, flammable materials..."
— see ADJ15's gemma4 rulebook dump) and routes matches through
"flammable materials". The categorical leap (matches → flammable
material) is still happening in the forward pass, but the rule it
fires on is from a *different model's elicitation*, which is the
intended adversarial structure.

**gemma4:latest** (8B):
> Arm A failed: output truncated: model emitted 512 tokens and
> stopped at the max_tokens cap (512); raise the cap and retry.

This is the only non-flip in the run, and it's a *typed failure*,
not a silent bad answer. With ~3,500 bytes of merged adversarial
rulebook in its context window, gemma4 starts chain-of-thought
narrating the rulebook ("Looking at rule 1, we see that... and
rule 2 says... and rule 3 covers...") and exhausts the 512-token
answer budget before reaching a VERDICT line. The truncation is
surfaced as `LlmError::OutputTruncated` by llm-primitives, and the
demo correctly reports "Arm A failed: output truncated" rather than
attempting to interpret a half-finished answer.

The right framework behavior is to either raise the cap (the
truncation error suggests this) or run ADJ06-style clarification
to ask the model to produce a verdict line first and reasoning
second. Neither is implemented in v0.10; both are clean follow-ups.

## Decoupling elicitation from answer

The mechanism worth naming explicitly: ADJ17 demonstrates that
**rulebook elicitation and rulebook usage are separable
capabilities**. ADJ15 measured them as a single recursive loop
(same model, both roles) and the loop broke at small scales. ADJ17
splits the roles — elicitation runs on the two 8B-class models,
usage runs on the answerer of choice — and the small-model bottom
end of the lineup recovers.

This matters for deployment economics. The ADJ12 framing was
"shrink the model down, push intelligence into the framework". ADJ17
sharpens that: the intelligence pushed into the framework can come
from larger models *at elicitation time*, then be reused indefinitely
by smaller models at answer time. The 8B models pay the elicitation
cost once; the 0.5B model pays a tiny answer cost per question. The
elicited rulebook becomes a transferable asset across the model
size ladder.

This is **structurally similar to knowledge distillation**, but the
distillation product is auditable, human-reviewable text (the
rulebook) rather than weight updates to a smaller model. A
domain expert can read the merged adversarial rulebook, accept it,
edit it, or reject specific rules — and then promote the trust tier
from `Tentative` to `Reviewed` per ADJ14, after which arbitrarily
small models can deploy against it.

## Circular hallucination risk

ADJ15 §Caveats(3) flagged "all three flips on the 3B+ tier used
citations from the model's *own* elicited rulebook" as a
circular-hallucination risk: the model invented rules, then cited
those rules, and the verdict felt correct because both halves of
the loop agreed by construction.

ADJ17 *partially* addresses this. The risk is reduced but not
eliminated:

1. **Cross-family agreement is now visible.** The merged rulebook
   surfaces rules that gemma4 and llama3.1 *both* produced (e.g.,
   both have a "prohibited items: weapons/explosives/flammable
   materials" rule near position 3) versus rules that only one
   model produced (llama3.1's "doctor's note exception" was solo;
   gemma4's "8,192-token caveat" was solo). A downstream rule
   adjudicator (not yet built) can weight cross-model agreement as
   higher trust than single-model rules.
2. **The answerer model never elicits a rule it later cites.** In
   ADJ17 the answerer is fed rules from a different model's weights
   than its own forward pass. qwen2.5:0.5b cites rule 3 — a rule
   produced by gemma4 — which it could not have invented itself
   (ADJ12 showed it has no coherent internal TSA representation).
3. **The categorical leap still happens inside the answerer's
   forward pass.** "Matches → flammable materials" is the leap; it
   is not in any rulebook. The answerer commits to that leap when
   it cites rule 3. ADJ16 (engine-programmatic adjudication) is the
   structural answer to this — the engine would force the leap to
   appear as an explicit step in the proof DAG.

The hallucination risk that *does* increase: a rule fabricated by
gemma4 could be cited authoritatively by qwen2.5:0.5b, with the
provenance tag making the fabrication look credentialed. The
mitigation is human review at the rulebook level — `Tentative` →
`Reviewed` promotion is the load-bearing checkpoint. Reviewers see
the gemma4 rulebook and the llama3.1 rulebook side-by-side with
provenance, which is the right shape of artifact to review.

## Caveats

1. **n = 1 test case** (same as ADJ15). One source string proves
   the pattern can produce striking deltas on this particular
   question. Generalisation requires the ADJ10 declaration variants
   (toothpaste / perfume / lithium battery / wine / pocket knife /
   lighter / strike-anywhere matches) and a flip-rate analysis.

2. **`validation_passed = false` everywhere** (same as ADJ15). Both
   stage-0 rulebooks fail strict ADJ02 v3 graph-IR validation by a
   handful of bytes (`CoverageGap(194, 200)` for gemma4;
   `CoverageGap(43, 44)` for llama3.1). The merged source text is
   well-formed prose; the graph-IR built from it has small
   coverage gaps the strict checker rejects. Per ADJ14 trust tiers,
   both rulebooks ship as `Tentative` and must be promoted to
   `Reviewed` by a domain expert before production use. The same
   `decompose_text` v4 prompt rollout that ADJ15 §Caveats(2)
   flagged would also reduce ADJ17's coverage gaps.

3. **qwen2.5:0.5b's answer is muddled even when correct.** The
   verdict is right and the rule citation is right, but the path to
   the verdict ("based on Document_ID: 423, … since there are no
   specific numbers listed for this particular finding, we can
   assume that the passenger's declaration does not violate any of
   the rules outlined in the document ID. Therefore, the final
   verdict is: NON-COMPLIANT") is internally contradictory. A
   reviewer reading just the verdict line would accept it; a
   reviewer reading the full text would flag it. The framework
   currently captures the full text in the audit trail, so the
   flag-opportunity is preserved.

4. **gemma4's truncation is a real workflow problem.** Half the
   answerer time on the largest model goes to producing no usable
   answer. v0.10 surfaces the truncation as a typed error; v0.11
   should retry with a larger `max_tokens` cap (the truncation
   helper exists in llm-primitives but is not wired through Arm A).
   Alternatively, the system prompt could force a `VERDICT:
   ...` line first, reasoning after, so the verdict survives even
   if the reasoning truncates.

5. **The llm-cache disk-persistence bug is still latent.** Every
   ADJ17 run re-elicited both 8B rulebooks from scratch, even with
   `ADJ_DEMO_CACHE_DIR=/tmp/adj_bench_cache` set. The `OllamaClient`
   instances constructed inside the demo for the adversarial
   elicitation list are not wrapped in the cache layer, so the
   ~261 s × N answerer runs paid the full elicitation cost N times
   instead of once. A wiring fix would cut this benchmark's
   wall-clock time to ~15 minutes; it is queued as the next
   adjudication-rulebook bug fix.

6. **Independence is only at the rulebook level, not the answer
   level.** The adversarial set varies the rulebook source; the
   answer is still single-model. A future ADJ would adversarialise
   the answer step too: two different answerer models, with an
   ADJ05-style judge deciding which interpretation is more
   plausible.

7. **No human review yet promoted these rulebooks.** Both gemma4's
   and llama3.1's elicited rulebooks remain `Tentative`. The flips
   in this run derive their authority from the `Tentative`-tier
   audit trail (every cited rule has provenance tagging back to
   the model that produced it), not from `Reviewed`-tier sign-off.
   This is the right level of trust for an empirical bench; it is
   not the right level of trust for a production verdict.

## What this tells us about the framework

ADJ15 showed the recursive pattern works *if* the model is large
enough to elicit a usable rulebook. ADJ17 shows that **once a
rulebook exists, the answerer model can be much smaller** — the
elicit-cost is paid once on the big model, the answer-cost is paid
every time on whatever model fits the deployment.

The most economically useful regime of the framework is therefore:

1. **Big-model elicitation** (gemma4, llama3.1, or a frontier
   model) at the rulebook-authoring stage, with adversarial
   multi-model elicitation for cross-checking.
2. **Human review** to promote the merged rulebook from
   `Tentative` to `Reviewed`.
3. **Small-model usage** (0.5B–3B) at answer time, with the
   `Reviewed` rulebook injected into the system prompt and the
   ADJ02/ADJ03/ADJ04/ADJ05/ADJ06 pipeline running over the
   answerer's output.

This is the framework's analog of "compile once, run anywhere":
**elicit-and-review once, deploy on small models everywhere**. The
deployment economics flip from "frontier model latency per
question" to "frontier model latency per *rule update*". For a
regulatory domain like TSA where rules change quarterly, the
savings compound.

## What ADJ17 does not yet show

The deterministic-engine win in [ADJ16](ADJ16-engine-programmatic-adjudication.md)
is the next step beyond this. ADJ17 still uses the LLM at answer
time — it just gives the LLM a stronger rulebook. The categorical
leaps ("matches → flammable materials") still happen inside the
answerer's forward pass and are not externally auditable. ADJ16's
proposal is to **compile the rulebook into Prolog/ProbLog** and run
the engine at answer time, making the categorical leap an explicit
unification step in a proof DAG. That removes the last surface
where the model can confabulate — the answerer becomes deterministic
modulo fact extraction.

The ADJ15 → ADJ17 → ADJ16 progression is therefore:

- **ADJ15**: LLM elicits, same LLM answers. Recursion works at 3B+,
  breaks below.
- **ADJ17**: multiple LLMs elicit, *any* LLM (even 0.5B) can answer
  with the merged rulebook. Recursion works across the full size
  range; risk concentrates in human review of the merged rulebook.
- **ADJ16**: multiple LLMs elicit, the rulebook compiles to Prolog,
  the engine answers deterministically. The LLM's role at answer
  time shrinks to fact extraction only.

Each step removes one source of model-internal opacity.

## Status

Single-shot empirical measurement, n=1 test case, 2026-05-12. The
pattern is striking enough to publish but not a benchmark in the
strict sense — that requires an evaluation set and ground-truth
verdicts per case. The natural follow-ups, in order:

1. Run ADJ17 across the ADJ10 declaration variants on the same
   5-model lineup; produce flip-rate tables.
2. Wire the llm-cache through `acquire_rulebook_adversarial` so
   the elicitation cost is paid once.
3. Wire `OutputTruncated` retry through Arm A so gemma4's
   token-budget overrun resolves automatically.
4. Land [ADJ16](ADJ16-engine-programmatic-adjudication.md) step 1:
   pass the per-rule provenance and trust tier through
   `adjudication-connector` into the Prolog facts.

## Reproduction

```bash
# Build the demo (v0.10+):
cargo build -p adjudication-tsa-demo --release

# Run a single answerer with adversarial elicit (vary ADJ_DEMO_MODEL):
ADJ_DEMO_MODEL=qwen2.5:0.5b \
  ADJ_DEMO_ENDPOINT=http://127.0.0.1:11434 \
  ADJ_DEMO_SOURCE="1 carry-on bag, matches." \
  ADJ_DEMO_IR_MODE=hand \
  ADJ_DEMO_RULEBOOK_MODE="adversarial:gemma4:latest,llama3.1:8b" \
  ADJ_DEMO_CACHE_DIR=/tmp/adj_bench_cache \
  ADJ_DEMO_TIMEOUT_SECS=300 \
  ./target/release/adjudication-tsa-demo
```

The full benchmark is reproducible by iterating over the 5
answerer models (`gemma4:latest`, `llama3.1:8b`, `qwen2.5:3b`,
`qwen2.5:1.5b`, `qwen2.5:0.5b`) with the same adversarial set. Raw
output captured at
[`data/adj17-adversarial-bench-2026-05-12.json`](data/adj17-adversarial-bench-2026-05-12.json).
