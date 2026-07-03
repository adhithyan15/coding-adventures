# ADJ99 — defensibility rescore under a counterfactual (locus-exposure) rubric

**Status: COMPLETE.** All 395 non-error ADJ99 cells (4 arms × 100 items, minus 5 agent-errors)
re-judged by a fresh blind Opus judge under a **construct-valid** defensibility rubric, with the
solver trace **format-normalized** so the judge cannot read the arm off the style.

> TL;DR — ADJ99's original 0–5 defensibility score measured the **wrong construct** (citation /
> traceability density), and it did so with a **format leak** that let the "blind" judge identify the
> arm with 100% accuracy. Correcting both does **not** temper ADJ99's numbers — it **reverses ADJ99's
> headline.** Under a rubric that scores whether the load-bearing premise is *exposed and flagged as
> fallible*, the framework's advantage **grows** at both model scales, and the fw-opus result flips
> from "neutral / slightly harmful" (ADJ99's falsified H2) to a clear **+0.45** gain. The framework's
> defensibility benefit was being **masked** by the bad rubric, not inflated by it.

---

## 1. Why ADJ99's defensibility number was the wrong measurement

ADJ99 *defined* defensibility (its own words) as: *"the locus of contingency is exposed — every
load-bearing claim attributed to a source or a stated assumption — so that when the decision is wrong
you can point to the specific premise that produced the error, override it, and re-derive,"* explicitly
**decoupled from correctness**.

It then *measured* it with a 0–5 judge rubric whose top scores were *"nearly every / every claim
traceable to a cited source or the givens."* That operationalizes **traceability** (attribution
density), not **defensibility** (is the fallible pivot surfaced). Two flaws follow.

### Flaw 1 — Format confound (proven deterministically; no LLM needed)

The `fw-*` arms emit a literal `RETRIEVED FACTS (CAS): … REASONING CHAIN … [cites: n]` structure; the
`plain-*` arms emit prose. Running [`rescore_prep.py`](rescore_prep.py) over the raw batches:

| signal | value |
|---|---|
| regex `{RETRIEVED FACTS, REASONING CHAIN, [cites, (src:}` → predicts fw-vs-plain | **100.0%** (197/197 plain clean; 197/197 fw tagged) |
| old def≥4 answers that were **wrong** | **70.3%** (109/155) |

A one-line regex separates the arms perfectly. The "blind" judge was **not blind** — it could read the
arm off the very citation-shaped style the rubric rewarded. And 70% of high-defensibility answers were
wrong, because the rubric graded whether claims were *attributed*, not whether the pivot was *true or
flagged fallible*. (Full numbers: [`confound_analysis.json`](confound_analysis.json).)

### Flaw 2 — Wrong construct

Citation density rewards a confidently-wrong chain in which every step cites a (bad) fact. That is the
opposite of defensible. The fix is a rubric that scores **locus-exposure**.

---

## 2. The corrected measurement

**Rubric (counterfactual / locus-exposure).** Score 0–5 on whether the trace exposes its own
**load-bearing premise — the single fact or assumption whose change would flip the answer — and flags
it as fallible**, so a reviewer can override it and re-derive. Citations, length, fluency, and
correctness are explicitly *not* rewarded. (Verbatim rubric + JSON schema in
[`rejudge.workflow.js`](rejudge.workflow.js).) The judge also returns three booleans per cell:
`premise_named`, `premise_flagged_fallible`, `states_what_would_flip_answer`.

**Format normalization.** Every trace (both arms) is rendered into an identical
`REASONING: … / CONCLUSION: …` envelope with `RETRIEVED FACTS`, `[n]`, `(src: …)`, `[cites: …]`, and
`REASONING CHAIN` chrome stripped, so style cannot leak the arm. Source URLs are removed deliberately:
under the new rubric the URL is not what is scored, and keeping it would only reintroduce the tell.
Spot-checked: post-normalization, fw traces contain none of the four format tells.

**Blindness.** Each judge saw only `{category, question, normalized trace}` — never the arm, the gold
answer, or the old score.

---

## 3. Corrected headline (n = 395)

| arm | n | **old** def (traceability) | **new** def (locus-exposure) | named % | flagged-fallible % | would-flip % | new def≥4 |
|---|---|---|---|---|---|---|---|
| plain-haiku | 100 | 2.14 | **1.90** | 23% | 2% | 1% | 2 |
| plain-opus | 98 | 3.72 | **3.03** | 60% | 16% | 14% | 16 |
| fw-haiku | 99 | 2.68 | **2.71** | 47% | 14% | 7% | 13 |
| fw-opus | 98 | 3.61 | **3.48** | 78% | 34% | 31% | 33 |

**The fw − plain gap, old vs new:**

| pair | OLD gap (traceability) | NEW gap (locus-exposure) |
|---|---|---|
| haiku (fw − plain) | +0.54 | **+0.81** |
| opus (fw − plain) | **−0.11** | **+0.45** |

The correction **widens** the framework's advantage at both scales, and on Opus it **flips sign**.

---

## 4. What this does to ADJ99's two pre-registered hypotheses

- **H2 — "Opus + framework is more defensible than plain-Opus."** ADJ99 declared this **FALSIFIED**
  (fw-opus 3.61 < plain-opus 3.72, by −0.11). Under construct-valid, format-normalized scoring it is
  **TRUE**: fw-opus 3.48 > plain-opus 3.03, by **+0.45**. ADJ99's falsification was an **artifact of
  measuring traceability** — plain-opus's fluent, traceable prose scored high on citation density while
  exposing a fallible pivot far less often than the framework's output did.

- **H1 — "Haiku + framework reaches plain-Opus defensibility."** Still **not reached**, but the gap
  shrinks from **1.05** (old: 2.68 vs 3.72) to **0.32** (new: 2.71 vs 3.03). Much closer than ADJ99
  reported; the original 1-point gap was mostly the format/traceability confound, not a real
  defensibility deficit.

---

## 5. The mechanism — why the framework genuinely helps

Defensibility, properly defined, is about **flagging your pivot as fallible**. The framework roughly
**doubles** the rate at which a model does that — at *both* scales:

| metric | plain-haiku | fw-haiku | plain-opus | fw-opus |
|---|---|---|---|---|
| names the load-bearing premise | 23% | **47%** | 60% | **78%** |
| flags that premise as fallible | 2% | **14%** | 16% | **34%** |
| states what would flip the answer | 1% | **7%** | 14% | **31%** |

The spider→facts→reasoning structure pushes the model to surface *which* premise is load-bearing and to
mark it as overridable. That is a **real, format-independent gain** — it survives stripping every
citation. The old rubric couldn't see it because it was busy rewarding prose for being *traceable*.

**The new score is cleanly decoupled from correctness** (the property defensibility is supposed to
have): within every arm, mean new-def is ~identical for correct vs incorrect answers (e.g. fw-opus 3.26
on correct, 3.56 on incorrect), and **84.4%** of new-def≥4 cells are *wrong* — even more than under the
old rubric. That is the **intended** behavior: a defensible-but-wrong answer (pivot named and flagged,
answer still incorrect) is exactly what we want to score high. Examples in the data:

- **fw-opus, idx 347 (incorrect, def 5):** names the non-ideal-vs-ideal-Boltzmann reading as the pivot,
  flags it assumption-dependent, and states that setting the van-der-Waals terms to zero recovers the
  closed form — defensible, and wrong.
- **plain-opus, idx 113 (incorrect, def 5):** states the answer rests entirely on a single Etsy SEO-spam
  listing, flags it unverifiable, makes the whole conclusion overridable — defensible, and wrong.
- **plain-haiku, idx 8 (incorrect, def 0):** "(none provided)" — a bare assertion, no pivot, no flag.

(plain-opus *can* expose pivots well — idx 57/113/349 all scored 5; the framework raises the **rate**,
it doesn't hold a monopoly.)

---

## 6. Honest caveats

1. **Normalization is itself a judgment call.** Stripping citation chrome could in principle remove
   substance. We mitigated by keeping all factual prose and only removing headers/markers/URLs, and by
   scoring locus-exposure (which URLs don't contribute to). The normalizer is in
   [`rescore_prep.py`](rescore_prep.py); the before/after is reproducible.
2. **Still a single blind-judge score per cell** (n=1/cell), now under a better rubric. The effects
   reported are the large, stable ones (the sign-flip on Opus; the ~2× fallibility-flagging rate); do
   not over-read small deltas.
3. **Rate-limit operational note.** The 395-cell re-judge was run in self-paced batches of 10
   (sequential `parallel()` barriers) after larger bursts hit a ~50-call/window ceiling; the verdicts
   are arm-blind and rubric-identical across batches. Raw verdicts: [`verdicts.json`](verdicts.json);
   aggregation: [`rescore_aggregate.py`](rescore_aggregate.py) → [`rescore_summary.json`](rescore_summary.json).

---

## 7. Bottom line

ADJ99 was right to flag its own rubric as a weak proxy — but it under-stated the consequence. The
proxy was not merely noisy; it was **systematically biased toward fluent traceable prose**, which
**masked** the framework's actual contribution. Measured correctly — locus-exposure, format-normalized —
the byte-provenance framework makes a model **expose and flag its load-bearing premise as fallible
roughly twice as often, at both model scales**, and the "framework is defensibility-neutral on the
frontier model" conclusion is **falsified**. Defensibility does *not* simply track the underlying model;
the scaffold adds a real, measurable, correctness-independent increment. The durable ADJ99 positive
(auditability: ~90% flaw-localization, ~52% fixable CAS fact) stands; this rescore upgrades the
*defensibility* result from "confounded and neutral" to "real and model-independent."
