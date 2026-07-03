# ADJ72 — 50-sample blind run: findings

50 claims, 3 independent blind closed-book resamples each (12 subagent batches),
scored at the strict **3-of-3 identical** bar. Pre-registered categories:
A = canonical/exact (17), B = translation-or-edition-variable (16),
C = confabulation trap (17). Subagents were permitted to answer "no verbatim
source exists" so honest backtracking is captured rather than forced fabrication.

## Stability bucket × pre-registered category

| bucket | A | B | C | total |
|---|--:|--:|--:|--:|
| STABLE_3of3 (identical bytes ×3) | 17 | 9 | 3 | 29 |
| SPLIT_2of3 (2 agree, 1 differs) | 0 | 7 | 1 | 8 |
| UNSTABLE (all 3 differ) | 0 | 0 | 2 | 2 |
| STABLE_NULL (all 3 say "no verbatim source") | 0 | 0 | 11 | 11 |

## Headline numbers

- **Invention rate among STABLE_3of3 claims: 0 / 29.** Every claim that came back
  byte-identical across three independent resamples was a *genuinely real passage* —
  not one fabrication. Byte-stability cleanly separated retrieval from invention.
- **All 17 canonical (A) claims → STABLE_3of3, all correct.** Perfect.
- **Trap rejection: 16 / 17 (C).** Of the 17 traps, 11 were consistently rejected as
  apocryphal (STABLE_NULL — the model said "no such verbatim source" all three times),
  2 were correctly flagged unstable, 1 split, and 2 were correctly answered
  (Q35 Armstrong's transmitted words; Q47 — the model gave the *accurate* Twain wording
  and resisted the popular misquote). Exactly **one** trap produced a dangerous result.
- **The one genuine failure — Q41 (Watson–Crick "closing sentence"):**
  STABLE_3of3, confidence 0.85–0.97, `exists=true` ×3 — and **wrong for the question.**
  The model reliably returned *"It has not escaped our notice that the specific pairing
  we have postulated immediately suggests a possible copying mechanism for the genetic
  material."* That sentence **is real and genuinely in the paper** — but it is **not the
  closing sentence** (the paper ends with acknowledgements). So the bytes are authentic;
  they just don't answer the precise question asked.

## The sharpened lesson

The n=4 probe said "stability is a detector, not a selector." The 50-sample run sharpens
it further and, importantly, **mostly vindicates the closed-book bet**:

> **Byte-stability reliably certifies that a passage is genuinely retrieved from the
> model's memory rather than invented (0/29 inventions here).** What it does NOT certify
> is that the retrieved passage *answers the specific question*. The residual failure
> mode is **relevance/precision (Q41: a real sentence misapplied as "the closing
> sentence"), not hallucination.**

This is a meaningfully stronger result than the probe suggested. Across 50 mixed claims —
17 of them deliberately designed to bait fabrication — the closed-book resampling
discipline produced **zero stable fabrications**. The model's own consistency separated
"I have this" from "I'm making this up," and where it genuinely lacked a source (11
apocryphal traps) it consistently *said so* rather than inventing.

## On the user's bet

The bet was: "the model already has this information and is simply missing it." The run
supports a precise version:

- When the model **has** it → resampling surfaces the **same** bytes (29/29 stable were
  real). The knowledge was there; consistency reveals it.
- When the model **lacks** a real source (apocryphal traps) → resampling **stably
  reveals the absence** (11/11 NULL), not a fabrication.
- The gap is **specificity**: the model can stably retrieve the *right kind* of passage
  (a real famous line) yet miss the *exact* one a precise question pins down (Q41).

So a closed-book grounding mode is viable as a **confabulation filter**: STABLE_3of3 +
`exists=true` is strong evidence the bytes are real and can be cheaply CAS-confirmed once.
The remaining check that stays necessary is **relevance** — does the (real) passage
actually satisfy the question — which a one-time open-book confirmation (ADJ71) settles.

## Honest caveats

- The 3 resamples within a round were batched (one subagent answered ~13 claims);
  cross-round comparison is the independence axis. A fully clean design spawns one
  subagent per (claim, resample) = 150 calls. This run used 12.
- Correctness scoring: all A claims and the stable B/C claims were spot-verified
  (against known text + the Watson–Crick and Darwin web checks from the probe); the 11
  NULL traps were scored against well-established misattribution facts. A full
  per-claim open-book verifier pass was not run.
- The `confidence` field is muddy for NULL claims (confidence in a passage that doesn't
  exist) — some NULLs reported high confidence meaning "confident it's apocryphal," some
  low meaning "no passage." Stability, not self-confidence, was the reliable signal.
- Single model family (Claude). The stronger test adds a different-family resample arm
  (ADJ05 independence): cross-model byte agreement.

## Bottom line

On a 50-claim spectrum built to bait it, closed-book byte-stability across independent
resamples produced **0 stable fabrications, caught 16/17 traps, and flagged every
canonical fact as solid.** The one miss was a real-but-misapplied passage — a relevance
error, not a hallucination. The detector works; its boundary is precision, not honesty.
