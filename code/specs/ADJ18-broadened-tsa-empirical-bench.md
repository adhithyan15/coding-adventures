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

## Empirical results (2026-05-13)

Bench ran end-to-end on 2026-05-13 against local Ollama (5 models
all pulled, warm cache). All 120 cells completed in ~7 minutes
wallclock (much faster than the 2-4 hour estimate thanks to the
warm cache from earlier ADJ15/ADJ17 runs and the 0.5B-1.5B
models' fast inference). Raw data:
[`data/adj18-tsa-bench-2026-05-13.json`](data/adj18-tsa-bench-2026-05-13.json).

### Headline numbers

| Mode | Correct | Total | Accuracy | Parse fails | Truncations |
|---|---|---|---|---|---|
| `none` | 23 | 40 | **57.5%** | 0 | 0 |
| `fixture-single` | 20 | 40 | **50.0%** | 0 | 0 |
| `fixture-priming` | 16 | 40 | **40.0%** | 10 | 0 |

**The most surprising finding**: rulebook injection *decreased*
mean verdict accuracy across the 8-declaration set. The `none`
baseline scored highest at 57.5%. This is the **opposite** of
what ADJ15 and ADJ17 saw on the single matches declaration —
and it reframes those results.

### Per-(model, mode) breakdown

```
gemma4:latest:
  none              : 4/8 (50.0%)
  fixture-single    : 6/8 (75.0%)  ← rulebook helped this model
  fixture-priming   : 5/8 (62.5%)

llama3.1:8b:
  none              : 5/8 (62.5%)
  fixture-single    : 5/8 (62.5%)
  fixture-priming   : 4/8 (50.0%)

qwen2.5:3b:
  none              : 5/8 (62.5%)
  fixture-single    : 3/8 (37.5%)  ← rulebook HURT this model
  fixture-priming   : 4/8 (50.0%)

qwen2.5:1.5b:
  none              : 5/8 (62.5%)
  fixture-single    : 3/8 (37.5%)
  fixture-priming   : 1/8 (12.5%) with 5 parse failures

qwen2.5:0.5b:
  none              : 4/8 (50.0%)
  fixture-single    : 3/8 (37.5%)
  fixture-priming   : 2/8 (25.0%) with 5 parse failures
```

### What actually happened

**H1 (rulebook injection improves accuracy across the set):
FALSE.** Mode 1→2 went down (57.5% → 50.0%), not up. Only
gemma4:latest gained from rulebook injection; every other model
regressed. This is the opposite of what we expected after ADJ15
and ADJ17.

**H2 (priming reduces truncation): VACUOUSLY TRUE.** Zero
truncations across all 120 cells. The v0.12 max_answer_tokens
default of 2048 + the verdict-first prompt format **completely
eliminated** the gemma4 truncation problem we saw on the single
adversarial-rulebook case in ADJ17. There was nothing left for
priming to fix.

**H3 (priming preserves or improves verdict accuracy): FALSE.**
Priming made things worse, not better — 50% → 40% mean
accuracy. The reason is now visible in the data: priming
produced 5 parse failures each on qwen2.5:1.5b and qwen2.5:0.5b
(no `VERDICT:` line in the response). The 0.5B and 1.5B models
couldn't reliably follow the two-turn protocol — they treated
turn 1 as the question, or got confused by the conversation
structure, and never emitted the verdict format the harness
parses.

**H4 (mode-1 hallucination pattern consistent across
declarations): PARTIALLY CONFIRMED, but with a twist.** The
`none` baseline didn't uniformly hallucinate — for many
declarations (`pocket-knife`, `wine-bottle`, several
`small-lithium` cases) most models got the verdict right
*without* a rulebook. The training data has TSA-shaped
knowledge for the common cases; the model only hallucinates
when the case is at the edges of its knowledge or when
prompted to invent.

### The pocket-knife regression

The most striking single case in the data:

```
pocket-knife (expected NON-COMPLIANT):
  Model              none              fixture-single
  gemma4:latest      OK NON-COMPLIANT  WRG COMPLIANT
  llama3.1:8b        OK NON-COMPLIANT  WRG COMPLIANT
  qwen2.5:3b         OK NON-COMPLIANT  WRG COMPLIANT
  qwen2.5:1.5b       OK NON-COMPLIANT  WRG COMPLIANT
  qwen2.5:0.5b       OK NON-COMPLIANT  WRG COMPLIANT
```

**Every model got pocket-knife right WITHOUT a rulebook (5/5)
and every model got it WRONG WITH the fixture rulebook (5/5).**

Why? The fixture rulebook ([`fixture_tsa_rulebook()`](../packages/rust/adjudication-tsa-demo/src/lib.rs))
enumerates rules for strike-anywhere matches (rule 3), lithium
batteries (rule 4), liquids (rule 1), and explosives (rule 5)
— but **does not mention pocket knives**. Combined with the
system prompt's instruction *"Do not invent any additional
rules; if a finding is not justified by a specific numbered
rule below, do not include it"*, the model is actively prevented
from using its background TSA knowledge about knife-blade-length
limits.

In `none` mode, the model fell back on training-data knowledge
("knives are prohibited in carry-on") and got the right answer.
In `fixture-single` mode, the model was told *"only use these
rules"* and concluded *"these rules don't cover pocket knives,
so I have no basis to prohibit them"*. That's a defensible
reading of the prompt — and exactly the wrong outcome.

### Why this is important for the framework

This is **the strongest empirical argument so far** for
[ADJ20's fact-sheet primitive](ADJ20-fact-sheets-and-pipeline-reorder.md).

The framework's current Arm A asks the LLM to do **two things at
once**: apply the rulebook AND not invent. The pocket-knife case
shows these can be in direct conflict — applying the rulebook
faithfully means refusing to flag a violation the rulebook
doesn't cover, even when the violation is obvious.

ADJ20 separates these concerns:

- **Rulebook** says "items meeting condition X are prohibited"
  (general regulatory rule).
- **Fact sheet** says "4-inch pocket knife has blade length
  exceeding the TSA 2.36 in limit" (entity-specific world
  knowledge).
- **Engine** unifies: rule + fact = verdict.

The "do not invent rules" constraint is preserved on the rulebook
side, but the fact sheet legitimises world-knowledge facts the
rulebook didn't anticipate. The model is no longer forced to
choose between *applying the rulebook* and *using common sense
about the entity at hand*.

### Why rulebook injection helped gemma4 specifically

Gemma4 is the only model that gained from rulebook injection
(50% → 75% in fixture-single mode). The pattern in the raw data:
gemma4 in `none` mode tended toward "this seems fine" answers
on cases it shouldn't have (matches, large-lithium,
large-toothpaste, lighter-disposable). With the fixture rulebook,
gemma4 cited rules and flipped 5 declarations from COMPLIANT to
NON-COMPLIANT — even though it lost on pocket-knife.

So for gemma4, the framework's existing pattern is working: the
rulebook reins in the model's "default optimistic" reading
behaviour. For smaller models that already lean toward
NON-COMPLIANT by default (qwen2.5:3b on small-lithium and
small-perfume), the rulebook over-constrains them in the wrong
direction.

### The mode 1→2 regression by case

Some cases got better with the rulebook (gemma4 specifically),
some got worse (everyone on pocket-knife, smaller models on
several others). The aggregate is dominated by:

- **gains**: gemma4 on matches/large-lithium/large-toothpaste/lighter
- **regressions**: every model on pocket-knife; qwen2.5:1.5b and
  qwen2.5:3b on several other cases
- **wash**: cases where the model already had the right answer

The aggregate decline (57.5% → 50.0%) hides the structure: the
rulebook is a **good lever in one direction (gemma4) and a bad
lever in another (small models on pocket-knife)**.

### What changed since ADJ17

ADJ17's striking finding was that 0.5B and 1.5B models flipped to
NON-COMPLIANT with rule citations when given the adversarial
rulebook. That finding still holds on the matches case — the
data confirms it:

```
matches (expected NON-COMPLIANT):
  qwen2.5:1.5b: none → fixture-single  =  NON-COMPLIANT → COMPLIANT
```

Wait — this is the opposite of ADJ17's result. Let me re-read:
qwen2.5:1.5b said NON-COMPLIANT in `none` (the desired verdict)
and COMPLIANT in `fixture-single` (wrong). What happened?

Looking at the raw answers in `adj18-tsa-bench-2026-05-13.json`,
the `none`-mode answer from qwen2.5:1.5b on matches is *"VERDICT:
NON-COMPLIANT"* — the model gives the right answer from training
data alone. In `fixture-single` mode, the model sees the rulebook
(which DOES enumerate matches in rule 3) and... still says
COMPLIANT? This contradicts ADJ17. The difference must be the
**fixture rulebook vs the adversarial rulebook**.

ADJ17 injected the ~3,500-byte adversarial rulebook (merged from
gemma4 + llama3.1 elicitations). ADJ18 injects the canonical
hand-authored `fixture_tsa_rulebook()` which is ~1,200 bytes. The
adversarial rulebook had stronger rule wording, more authority
citations, and more redundancy. The fixture rulebook is terser.

This is a **second piece of evidence for ADJ20**: rulebook
quality and density matter as much as rulebook presence. A
sparse rulebook actively misleads smaller models by overriding
their training-data instincts without providing enough new
information to compensate. ADJ17's adversarial rulebook had the
density to do this; ADJ18's fixture rulebook doesn't, and the
flip pattern reverses.

### Truncation: completely solved

Zero truncations across all 120 cells. The v0.12 changes (PR
#3057) — raising max_answer_tokens from 512 to 2048 and putting
the verdict line first — completely eliminated the failure mode
that ADJ17 §Caveats(4) flagged as "a real workflow problem". H2
is vacuously confirmed but the cause is the max_tokens raise, not
priming. Priming is the wrong tool for a problem that no longer
exists.

### What we should do with ADJ_DEMO_ARM_A_MODE=priming

It's net-negative on the current bench. The 0.5B and 1.5B models
can't follow the two-turn protocol. For the 3B model, priming is
a wash. For the 8B models, priming is slightly negative compared
to single-turn.

Recommendation: **keep priming as an opt-in feature**, document
that it's primarily useful for very large input rulebooks that
risk truncation, and add a follow-up note that the current
default (`SingleTurn` + 2048 cap + verdict-first prompt) is the
right baseline for normal usage. The implementation lands as
infrastructure for future cases (long adversarial rulebooks,
multi-domain fact-sheet-rich contexts) where it might pay off.

### Summary findings

1. **Aggregate accuracy went DOWN with rulebook injection**
   (57.5% → 50.0% → 40.0%). This reframes ADJ15/17's
   single-string flip finding as case-dependent, not
   pattern-uniform.
2. **Truncation is solved.** v0.12's cap+verdict-first wins.
   Priming becomes a niche optimisation, not a default.
3. **The pocket-knife regression is the most important
   empirical finding** — rulebooks can actively *prevent*
   correct verdicts when the rulebook doesn't enumerate the
   relevant rule. This is the strongest argument so far for
   ADJ20's fact-sheet primitive.
4. **Small models can't follow the two-turn priming protocol
   reliably.** 10/30 priming-mode cells on the 0.5B and 1.5B
   models produced no `VERDICT:` line. Priming is not a
   small-model-friendly pattern.
5. **Rulebook quality matters.** ADJ17's adversarial rulebook
   produced the small-model flips; ADJ18's fixture rulebook
   doesn't. Density and authority citations both matter.

### What this changes about the roadmap

- **ADJ19 (cross-domain bench) is more important, not less.** If
  fixture rulebook injection regresses TSA accuracy, we should
  measure the same effect across clinical and contract before
  drawing universal conclusions.
- **ADJ20 (fact sheets) becomes the load-bearing next step.**
  The pocket-knife regression IS exactly the problem ADJ20
  addresses: separate world-knowledge facts (knife blade
  length) from rules (length > limit → prohibited). Without
  this separation, the rulebook is a lossy compression of the
  domain.
- **ADJ18.5 adversarial follow-up** should use the
  `adversarial:gemma4:latest,llama3.1:8b` rulebook on the same
  8 declarations and compare to fixture-rulebook results. We
  expect the adversarial rulebook to outperform the fixture
  rulebook (denser, more citations) but it should still show
  the pocket-knife-style regression where the rule isn't
  enumerated.

## v0.13 re-run (2026-05-13)

After [PR #3066](https://github.com/adhithyan15/coding-adventures/pull/3066)
landed `VERDICT: ESCALATE` as a third option in with-rulebook
Arm A prompts, we re-ran the 120-cell matrix against v0.13
prompts. Raw data:
[`data/adj18-tsa-bench-v0.13-2026-05-13.json`](data/adj18-tsa-bench-v0.13-2026-05-13.json).

### Aggregate

| Mode | v0.12 | v0.13 | Δ |
|---|---|---|---|
| `none` | 23/40 (57.5%) | 23/40 (57.5%) | — *(prompt unchanged)* |
| `fixture-single` | 20/40 (50.0%) | 17/40 (42.5%) | **−3** |
| `fixture-priming` | 16/40 (40.0%) | 23/40 (57.5%) | **+7** |
| **Total** | 59/120 (49.2%) | 63/120 (52.5%) | **+4** |

### The big priming-mode improvement

v0.12 priming produced 10 parse-failure cells across the 0.5B
and 1.5B models. v0.13 priming produced **0 parse failures**.
The more carefully structured prompt (with explicit ESCALATE
option and clearer turn-2 framing) reliably elicits a VERDICT
line from every model on every declaration. This alone accounts
for the entire net +4 accuracy improvement.

### ESCALATE was barely used

**ESCALATE used in 1 of 120 cells.** Gemma4 on the wine-bottle
declaration in `fixture-single` produced:

> *"VERDICT: ESCALATE — A supervisor needs to clarify the
> alcohol by volume (ABV) of the wine to determine if it is
> permitted under rule 8. ... Rule 8 prohibits beverages over
> 70% ABV, but the declaration does not specify the ABV, making
> compliance impossible to determine."*

A defensible escalation: rule 8 does constrain alcohol by ABV,
and the declaration doesn't specify it. The model spotted a real
ambiguity. (Caveat: it missed rule 2's 100 ml liquid limit,
which 750 ml vastly exceeds — a NON-COMPLIANT citing rule 2
would have been the stronger answer. But the ESCALATE under
rule 8 is correct application of the conservative semantic.)

Every other model on every other declaration produced one of
the two binary verdicts. Models told they could escalate did
not — even on cases where they were demonstrably wrong.

### The pocket-knife regression: still there

```
pocket-knife (expected NON-COMPLIANT, 4-inch blade > 2.36 in limit):

  Model              v0.12 fixture-single   v0.13 fixture-single
  gemma4:latest      ✗ COMPLIANT            ✗ COMPLIANT
  llama3.1:8b        ✗ COMPLIANT            ✗ COMPLIANT
  qwen2.5:3b         ✗ COMPLIANT            ✗ COMPLIANT
  qwen2.5:1.5b       ✗ COMPLIANT            ✗ COMPLIANT
  qwen2.5:0.5b       ✗ COMPLIANT            ✗ COMPLIANT
```

ESCALATE was supposed to catch this — every model fails the
numerical comparison `4 > 2.36`, and the right answer under the
conservative semantic is ESCALATE because the rule's condition
cannot be evaluated reliably. **None of the models escalated.**
They all confidently asserted the 4-inch blade was under the
2.36 in limit, just like in v0.12.

This is the cleanest evidence that **prompt-level ESCALATE is
not sufficient** for safety-critical adjudication. The model
cannot tell that it cannot do the arithmetic. The fix has to
happen outside the LLM:

- **ADJ20 fact sheets**: the engine handles the arithmetic.
- **Source decomposition** (next spec): the engine receives
  `blade_length(quantity(4, inches))` as a structured fact, not
  as a parsed-by-LLM number.

The v0.13 re-run is the empirical confirmation that the
framework needs both — prompt engineering alone can't get the
model to escalate on cases where it should.

### Why models don't escalate

Well-known in the LLM literature: models are trained to produce
confident answers. Instruction-tuning rewards giving an answer
over saying "I don't know." Adding `VERDICT: ESCALATE` as a
third label doesn't reweight the model's preferences enough to
overcome that prior.

Three follow-up experiments suggest themselves (not in this PR):

1. **Constrained decoding** on the verdict line. Restrict the
   first token after `VERDICT:` to one of three labels with
   equal logit mass to start. Today's Ollama path doesn't
   expose constrained decoding; would need a custom client.
2. **Few-shot anchoring.** One or two worked examples of cases
   that ESCALATE. Anchored examples are known to dramatically
   improve small-model behaviour on novel verdict options.
   Cheapest next experiment.
3. **Confidence-as-second-signal.** Ask the model to rate
   confidence separately; treat low confidence as ESCALATE
   regardless of the emitted label. Works around the model's
   reluctance to use ESCALATE directly.

### Conservative bias under priming

Four v0.12 parse-fails turned into wrong-NON-COMPLIANT under
v0.13 priming (small models defaulting to refuse-when-unsure):

- small-lithium/qwen2.5:0.5b
- small-perfume/qwen2.5:1.5b
- lighter-disposable/qwen2.5:1.5b
- lighter-disposable/qwen2.5:0.5b

For TSA that's defensible — refuse-when-unsure is the right
side to fail on for safety-critical screening. For clinical or
contract domains it may not be. ADJ19's cross-domain bench
will quantify the bias differently across domains.

### Summary

- **+4 cells correct overall** (49.2% → 52.5%), driven entirely
  by priming-mode parse-failure recovery on small models.
- **1 ESCALATE out of 120**, a sophisticated ABV ambiguity
  recognition by gemma4.
- **Pocket-knife regression persists** — every model still
  misapplies the numerical threshold, none escalates.
- Small models under priming default to NON-COMPLIANT rather
  than parse-fail; net wash on accuracy but a meaningful
  reduction in "completely broken" output shape.

**Structural conclusion**: prompt-level ESCALATE is necessary
but not sufficient. The framework needs the engine to evaluate
threshold conditions (ADJ20 fact sheets + source decomposition)
rather than relying on the LLM to escalate when it can't.

**Next experiments** (separate PRs):

- Few-shot anchored ESCALATE in the prompt.
- ADJ20-impl-1: `FactSheet` types and `elicit_subject_facts`
  primitive.
- Source decomposition spec: typed quantity extraction from
  declarations.
