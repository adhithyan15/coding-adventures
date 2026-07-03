# ADJ78 — a compact IR a 0.5B can build accurately, in steps

Goal: a compact intermediate representation for adjudicative work — **FACT (stated|inferred),
UNCERTAINTY, QUESTION** — that even a 0.5B local model can produce accurately, built
incrementally. Same shape for inputs and derived rulebooks.

## The design that works

**The model emits NATURAL content one kind at a time; the FRAMEWORK assigns type, status,
provenance, and structure.** The model never sees a schema, never tags, never emits offsets
— those are exactly what overloaded it (ADJ76/77).

- **Type** ← which step produced the line (FACTS step → FACT, etc.).
- **Status (stated|inferred)** ← PROVENANCE, not the model. A fact whose text anchors to a
  source span (fuzzy content-word match) is *stated*; one that doesn't is *inferred*.
  (v1 asked the model stated-vs-inferred and it answered badly — repeated stated facts in
  the inferred step. v2 deletes that question and derives status from the anchor.)
- **Provenance** ← deterministic fuzzy match of the phrase back to a source sentence.
  Hallucination guard: an unanchored "fact" is flagged inferred, never laundered as stated.
- **Byte-accounting** ← a gap-fill loop: every uncovered source sentence triggers ONE
  atomic call ("fact / question / discard? restate in one line"), typed by the choice and
  anchored to that sentence. One sentence at a time = tractable for a 0.5B (ADJ77).

## Results (qwen2.5:0.5b)

| passage | coverage before gap-fill | byte-accounted after gap-fill | stated facts | hallucinated facts |
|---|---:|---:|---:|---:|
| leave (4 sentences) | 1.00 | **1.00** | 3/3 anchored | 0 |
| clinic (5 sentences) | 0.40 | **1.00** | all anchored | 0 |

- **Stated-fact anchor rate = 100%, zero hallucinated stated facts** on both — the 0.5B's
  recorded facts are grounded to real source spans (the framework's match enforces it).
- The load-bearing override fact ("part-time hired after Jan 2020 → 12 days") is captured
  and anchored.
- The **gap-fill loop drove byte-accounting to 1.00** on the clinic passage (0.40 → 1.00):
  the 3 sentences the bulk steps missed were each resolved by a single atomic call
  ("recently traveled abroad" → FACT; "the team must decide…" → QUESTION).

## Honest limitations
- Coarse anchor-matching (content-word overlap): a couple of UNCERTAINTY/QUESTION nodes
  anchored to an approximately-related sentence rather than the exact one.
- One gap-fill restatement parsed garbled ("the sentence is a QUESTION") — cosmetic; the
  type and anchor were still correct.
- Question over-generation (7 questions on the clinic passage, some redundant) — needs a
  dedup/relevance trim.
- n=2 passages, one model. The stated/inferred-by-provenance rule needs validation where
  genuine inference occurs (here almost everything anchored, so inferred=0).

## Why this matters
This is the **foundational representation** for the small-local-model (airgapped/compliance)
deployment: a 0.5B can build a compact, typed, byte-accounted IR — covering facts,
uncertainties, and questions — when the framework owns all the structure and asks only
single natural questions. It is the substrate the reasoning scaffold (ADJ77) and the
provenance contract run on. Next: re-attach per-node justification, trim question
over-generation, and validate inferred-fact handling + rulebook-side IR (rules as
conditional FACTs).
