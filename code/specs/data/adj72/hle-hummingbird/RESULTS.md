# ADJ72 — hummingbird HLE question (the canonical frontier-model failure)

The flagship Humanity's Last Exam example. Reported bare-model answers: GPT-4o = 3,
Gemini 2.5 Flash = 2 — both confident, both wrong. Run blind through the full pipeline.

## What each stage produced

**Closed-book confabulation gate (3 resamples):**

| round | answer | verbatim source | exists | confidence |
|---|---|---|---|---|
| 1 | 8 | null | false | 0.20 |
| 2 | 7 | null | false | 0.15 |
| 3 | 7 | null | false | 0.15 |

Unstable (8/7/7), no verbatim passage, self-flagged `exists=false`, confidence ~0.15.
**The gate caught the ungroundedness.** Critically, the provenance demand changed the
model's behavior: the bare benchmark models shipped *confident* wrong numbers (2, 3);
asked to produce source bytes, Claude instead admitted it was guessing. The gate refused
to ship a number.

**Open-book grounding:** found the PRIMARY source — Zusi & Bentz (1984), *Myology of the
Purple-throated Carib and Other Hummingbirds*, Smithsonian Contributions to Zoology
No. 385 — downloaded the PDF, and grounded the count in a verbatim passage (p. 22):
"Flat tendinous fasciculi pass caudolaterally from the aponeurotic sheet and oval bone
to the bases of all but the medial rectrix." Hummingbirds have 10 rectrices = 5 pairs;
"all but the medial" = the 4 lateral pairs = **4 paired tendons**.

**Blind judge:**
- Bare closed-book (7/8, no source): **INCORRECT, undefensible.**
- Framework (4, Zusi & Bentz p.22 verbatim): **CORRECT, defensible.**

## Why this is the strongest run yet

Three distinct failure populations all get the wrong answer; only the framework gets it
right *and* defensibly:

1. **Bare frontier models** (benchmark): confident wrong — GPT-4o 3, Gemini 2.
2. **Bare Claude under provenance demand**: honestly unstable (8/7/7) — the gate's win is
   converting a confident guess into an admitted non-answer.
3. **Naive open-book / RAG**: every casual web source (Threads, askfilo, Course Hero, HLE
   commentary) parrots **"2"** with no citation. A retrieval system that trusted top
   search hits would confidently return the contaminated wrong answer.
4. **The framework**: confabulation gate flags ungroundedness → open-book layer rejects
   the parroted "2," hunts to the **primary anatomical monograph**, grounds **4** in a
   verbatim passage, and shows the rectrix-counting reasoning. Matches the documented HLE
   answer.

## The lesson

This is the clean case the byte-provenance discipline is built for, and all three
mechanisms fired:
- **Confabulation gate** (byte-stability + exists/confidence): caught that the closed-book
  answer was ungrounded *before* any number shipped — exactly where it succeeds (unstable
  confabulation, unlike the Palmyrene *stable* error).
- **Primary-source discipline**: the open-book layer's refusal to trust uncited secondary
  echoes ("2") is what separated it from naive retrieval. Contamination in the surface web
  is real; grounding-to-primary is what beats it.
- **Defensibility**: the framework's product isn't just the number 4 — it's the number 4
  with a verbatim Smithsonian passage and an explicit count, which an expert can verify in
  one click. The bare answer, even when a model happens to guess right, cannot offer that.

The framework helped, and the blind judge confirmed it: on the canonical question that
frontier models fail and the web contaminates, the assembled pipeline produced the
correct, primary-source-grounded, auditable answer.
