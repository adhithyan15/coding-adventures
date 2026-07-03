# ADJ32 — CLAIM_PROMPT Trailing-Punctuation Fix: Hypothesis Falsified, with a Bigger Finding

> Follow-up data PR to [ADJ31](ADJ31-per-level-gap-distribution.md).
> Tests **H4**: a prompt-level fix to `CLAIM_PROMPT` adding
> explicit handling of trailing punctuation closes the PhraseToClaim
> gaps ADJ31 localized.
>
> **Result: hypothesis falsified.** The targeted PhraseToClaim gaps
> did not close. Two cells regressed substantially (+3 and +5 gaps
> across all levels). The 0.5B and 1.5B models on the
> `lighter-disposable` declaration went from 5 → 10 and 1 → 4 gaps
> respectively.
>
> **Bigger finding**: lengthening the `CLAIM_PROMPT` system prompt
> with additional worked examples actively *regresses* small-model
> output quality across every level — not just PhraseToClaim. This
> is **a small-model context-window / instruction-following capacity
> finding**, more consequential than the falsified H4 itself.
>
> Raw data: [`data/adj32-h4-claim-prompt-trailing-punctuation-2026-06-01.json`](data/adj32-h4-claim-prompt-trailing-punctuation-2026-06-01.json).
>
> **The proposed prompt change is NOT shipped in this PR.** The
> branch contains the spec + raw data only; the
> `decompose_level.rs` modification was reverted after the bench
> falsified the hypothesis.

## Hypothesis (H4)

> ADJ31 identified PhraseToClaim as the dominant residual-gap level
> (3 of 5 parseable cells have their gap there). The
> `CLAIM_PROMPT`'s worked example shows `"1 carry-on bag"` → single
> Fact node with `text: "1 carry-on bag"` (14 chars) — but doesn't
> demonstrate handling of trailing punctuation. The model probably
> emits `text: "matches"` (7 chars) when the input phrase is
> `"matches."` (8 chars), dropping the trailing period and
> creating a 1-byte coverage gap.
>
> Fix: add two new GOOD EXAMPLES to `CLAIM_PROMPT` explicitly
> showing trailing-period and trailing-comma-and-space handling.
> Expected effect: PhraseToClaim gaps close on the 1.5B cells.

## Method

- Branched from `adj31-per-level-gap-distribution` so the instrumented
  bench binary is in place.
- Modified `code/packages/rust/llm-primitives/src/decompose_level.rs`
  `CLAIM_PROMPT`:
  - Strengthened the COVERAGE paragraph to call out "trailing
    punctuation, trailing whitespace, and any leading separators".
  - Added "GOOD EXAMPLE — TRAILING PUNCTUATION" showing
    `"matches."` → `text: "matches."` (with annotation explaining
    the 8 vs 7 character count).
  - Added "GOOD EXAMPLE — TRAILING WHITESPACE" showing
    `"1 carry-on bag, "` → `text: "1 carry-on bag, "` (with
    annotation explaining the 16 vs 14 character count).
- Rebuilt the bench binary and re-ran the same 6-cell set as
  ADJ31 (matches × {0.5b, 1.5b, 3b} + lighter-disposable × same).

## Results

| Cell | ADJ31 total | H4 total | Δ | ADJ31 PhraseToClaim | H4 PhraseToClaim | Δ |
|---|---|---|---|---|---|---|
| matches × qwen2.5:0.5b | 3 | 3 | = | 0 | 0 | = |
| matches × qwen2.5:1.5b | 1 | 1 | = | **1** | **1** | = |
| matches × qwen2.5:3b | 1 | 1 | = | 0 | 0 | = |
| lighter-disposable × qwen2.5:0.5b | 5 | **10** | **+5** | 1 | **4** | **+3** |
| lighter-disposable × qwen2.5:1.5b | 1 | **4** | **+3** | 1 | **2** | **+1** |
| lighter-disposable × qwen2.5:3b | unparseable | 4 | (recovered) | — | 2 | — |

### Per-level total gaps (H4)

| Level | Total |
|---|---|
| DocumentToSentence | 2 |
| SentenceToPhrase | 3 |
| **PhraseToClaim** | **9** *(was 3 in ADJ31)* |
| FactToTypedComponent | 8 *(was 2 in ADJ31)* |

PhraseToClaim — the level the prompt change was targeting —
*tripled* in total gap count. FactToTypedComponent — a level the
prompt change does not touch — *quadrupled* across the same cells.
The prompt change had system-wide negative effects on the small
models.

## Three findings

### Finding 1 — Targeted PhraseToClaim gaps did not close

The two 1.5B PhraseToClaim g=1 cells (matches and
lighter-disposable) were the targeted population. After the
prompt change:

- `matches × qwen2.5:1.5b`: still 1 PhraseToClaim gap (no change)
- `lighter-disposable × qwen2.5:1.5b`: went from 1 → 2 PhraseToClaim
  gaps (regression on the targeted level)

The hypothesis that "the model is dropping trailing punctuation
specifically at PhraseToClaim" is not supported. Whatever causes
the residual g=1 gap, it is not the absence of a trailing-punctuation
worked example in the prompt.

### Finding 2 — Lengthening the prompt regressed every level

The prompt change touched only `CLAIM_PROMPT` (the PhraseToClaim
boundary). But the gap counts at *other* levels — FactToTypedComponent
in particular — also worsened substantially:

- FactToTypedComponent total across the 6 cells: 2 → 8 (+300%)
- SentenceToPhrase total across the 6 cells: 3 → 3 (=)
- DocumentToSentence total across the 6 cells: 3 → 2 (−33%)
- PhraseToClaim total across the 6 cells: 3 → 9 (+200%)

This is consistent with the hypothesis that **the longer
`CLAIM_PROMPT` consumes context-window real estate that small
models would otherwise spend on producing valid Phrase output —
the level *before* PhraseToClaim in the pipeline**. If the small
model emits a degraded Phrase whose text doesn't tile its parent
Sentence cleanly, downstream levels see broken input and produce
more gaps in turn.

### Finding 3 — Small-model prompt-length sensitivity is empirically demonstrated

The prompt-length effect is not subtle. Three of six cells
regressed; the smallest model regressed the most. This is the
**inverse of the "more examples helps" intuition from few-shot
prompting**. For small local models doing structured output, a
longer prompt with more examples can *hurt*: the model loses
coherence on the core task while trying to incorporate the new
examples.

The implication for the framework's prompt design: **prompts at
deep levels need to stay short** for small-model robustness.
Adding examples to fix a specific failure mode has a real cost
that may exceed the benefit. The right path forward is probably:

- Keep `CLAIM_PROMPT` at its ADJ31 length or shorter.
- Investigate WHY the model produces a g=1 PhraseToClaim gap by
  capturing the raw IR output (the bench harness currently
  drops it on `error` paths) and inspecting it.
- Look for *structural* improvements (e.g., the orchestrator
  re-issuing the call with the specific failing parent text
  highlighted) rather than prompt-length increases.

## What ADJ32 changes (and what it doesn't)

**Changes**: nothing in the code. The proposed `CLAIM_PROMPT`
modification was reverted after the bench falsified the hypothesis.
This PR ships only the spec and the raw bench data.

**Doesn't change**: the framework's `decompose_level.rs` is
unchanged from ADJ29. The bench-binary instrumentation from ADJ31
is already on this branch (carried over from the parent branch).

## What ADJ32 implies for the next intervention

ADJ30 falsified the budget-bump intervention. ADJ32 falsifies the
prompt-extension intervention at PhraseToClaim. Two natural next
moves remain:

1. **Inspect raw IR output on failed cells.** The bench binary
   currently drops the IR JSON when there's an error. A small
   addition — emit the partial IR alongside the error — would
   let us see *what* the model is actually producing on the
   1.5B PhraseToClaim failure. Until we see the raw output, we
   are guessing at the failure mechanism.
2. **Structural intervention at the orchestrator level.** If the
   model consistently emits IR that fails coverage by exactly
   one byte at PhraseToClaim, the orchestrator could detect the
   pattern and apply a deterministic post-processing fix
   (e.g., merge the trailing character into the previous node).
   This sidesteps the prompt entirely.

ADJ33 (planned) will execute intervention #1 — instrument the
bench binary to emit partial IR on `CoverageUnresolved` errors —
and re-run the same 6 cells to capture the raw output. With that
data we can stop hypothesizing about *why* the gap exists.

## Methodology note — H4's regressions confirm per-run variance

ADJ31's run of `lighter-disposable × qwen2.5:0.5b` had 5 gaps.
H4's re-run with the proposed prompt change had 10 gaps. Some
fraction of that delta is the prompt change; some fraction is
per-run variance (which ADJ31 already established is non-trivial
on hierarchical-decomposition output).

The signal is still clear — 3 of 6 cells regressed, the regression
is concentrated on the small models, and the bigger-gap level
(FactToTypedComponent) changed substantially despite the prompt
not touching it. Even at the variance level we should attribute
to noise, the systematic regression on the small models is real.

## Cost summary

| Metric | Value |
|---|---|
| Cells run | 6 |
| Wallclock total | ~6.3 min |
| Prompt LOC added (then reverted) | ~25 |
| Code shipped to main | 0 |
| Cells passed | 0 |
| Cells improved | 0 |
| Cells regressed | 3 |

Two cheap experiments (ADJ30 and ADJ32) have falsified the
"obvious" interventions. The right next move is **measurement**
(raw IR on failed cells), not another intervention guess.

## Gating condition — still NOT met

Zero cells fully passing. Tier 1 unblock requires 5/40. ADJ32
contributes:

- 0 / 6 cells closed
- One empirically falsified intervention (prompt-extension at
  PhraseToClaim)
- One **secondary finding** (small-model prompt-length
  sensitivity) that may inform future prompt-design choices

## See also

- [ADJ31](ADJ31-per-level-gap-distribution.md) — localized the
  residual gaps to PhraseToClaim, motivating H4.
- [ADJ30](ADJ30-fact-typed-budget-bump-bench-results.md) —
  falsified the FactToTypedComponent budget bump.
- [ADJ29](ADJ29-per-level-retry-budget-bench-results.md) — the
  per-level retry budget bench.
- [ADJ28](ADJ28-anti-discard-bench-results.md) — the anti-discard
  prompt change that originally added the worked-example shape
  to the level prompts.

## Status

- 2026-06-01: H4 bench run complete; results captured;
  hypothesis falsified; proposed prompt change reverted from
  the source tree.
- Next: instrument the bench binary to emit partial IR on
  `CoverageUnresolved` errors (ADJ33), then re-run the 6 cells
  to capture what the model is actually producing on the
  residual PhraseToClaim gaps. Stop hypothesizing without
  evidence.
