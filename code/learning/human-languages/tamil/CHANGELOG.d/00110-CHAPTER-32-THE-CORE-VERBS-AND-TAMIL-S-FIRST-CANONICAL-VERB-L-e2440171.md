## Chapter 32 — the core verbs, and Tamil's first canonical `VERB-*` lessons

Tamil taught eighty-three lessons across thirty-one chapters and **four verbs**
— பேசு, வாழ், செய், போ — every one of them under a Tamil-only tag
(`TA-VERB-PESU`, `TA-VERB-VAZH`, …). A namespaced tag joins nothing across
languages, so on the cross-language measurement the track covered **zero** of
the forty canonical core verbs.

Chapter 32 adds six, one per lesson, in a prerequisite chain:

| lesson | word | concept |
|---|---|---|
| `TA-C32-iru` | இரு (*iru*) | `VERB-BE` |
| `TA-C32-po` | போ (*pō*) | `VERB-GO` |
| `TA-C32-vaa` | வா (*vā*) | `VERB-COME` |
| `TA-C32-saappidu` | சாப்பிடு (*sāppiḍu*) | `VERB-EAT` |
| `TA-C32-paar` | பார் (*pār*) | `VERB-SEE` |
| `TA-C32-teri` | தெரி (*teri*) | `VERB-KNOW` |

Sequences 610–660, all schema v2, all on `SPINE-SAY-WHAT-I-DO`. Tamil now
covers 6 of the core 40.

**The chapter is built around one thing: Tamil is agglutinative.** A verb is
beads on a string — stem, then tense, then person — so **இருக்கிறேன்** is
literally *be + present + I*, and the last bead already means "I," which is why
*nāṉ* can be dropped. That is a genuinely different machine from the European
verb, and it is the best hook the language offers a beginner.

One idea per lesson, each on the word that needs it:

- **இரு** — the three slots, named and assembled.
- **போ** — the middle bead alone carries tense: *pōgiṟēṉ / pōṉēṉ / pōvēṉ*. Its
  Kannada cousin **ಹೋಗು** (*hōgu*) is the same *p → h* softening Chapter 7
  already taught with *pattu* / *hattu*.
- **வா** — the form you call a verb by is not always the form the beads attach
  to: *vā!* as a command, but *varu-* under the suffixes. The farewell **போய்
  வருகிறேன்** from Chapter 4 has been carrying that form all along.
- **சாப்பிடு** — a verb Tamil assembled rather than inherited: சாப்பு (food) +
  இடு (to put), with இடு named as a **light verb**. Said honestly: the origin of
  சாப்பு itself is unsettled, and the inherited Dravidian eat-root did **not**
  die — it lives next door as Telugu *tinu*, Kannada *tinnu*, Malayalam
  *tinnuka*, and inside Tamil as தின் (narrowed to chewing) and literary உண்.
- **பார்** — the **strong/weak** split, on the one difference a learner has
  already heard four times: *pārkkiṟēṉ* doubles its *k* where *pōgiṟēṉ* keeps
  one, and strong verbs take a doubled *t* in the past (*pārttēṉ*). Two camps,
  learned with the verb. The family-wide see-root காண் is kept in view beside
  everyday பார்.
- **தெரி** — the closing symmetry. **தெரியும்** has no person bead at all, so
  the knower cannot climb into the last slot and rides in the dative instead:
  **எனக்குத் தமிழ் தெரியும்**, the sentence Chapter 6 gave whole and this lesson
  finally opens. Telugu builds it identically (**నాకు తెలుసు**).

Dravidian discipline held throughout: no Indo-European cognates are invented for
Tamil words, every cousin cited is a Dravidian sister with its form supplied, and
where a root is unsettled the lesson says so.

Drivability held deliberately: all six derive `voice`. No script blocks, no
sight cues, and the two tables are three wide, so the corpus's drivable count
rises by six with no new sight lessons. The letters each word needs are taught
inline in its *"Sounds you'll need"* block — every one of them was already met in
an earlier chapter, so nothing new had to be gated behind a reading section.

Wiring: `curriculum.json` gains `TA-PATH-028` on `SPINE-SAY-WHAT-I-DO` (the
track's first content above A1) with the six lessons attached as the required
`TA-EXT-028-CORE-VERBS` extension, and that node's `omits` ledger drops the six
concepts now realised. `chapters.json` gains a Chapter 32 entry whose payoff,
`TA-C32-teri`, assesses 7 of the chapter's 12 atoms (0.58, above the 0.5 floor).
`core/book-generation.json` gains the Chapter 32 target; the generated
`book/chapters/ch32-core-verbs.tex` is `\input` from `book.tex`, and the book
compiles under XeLaTeX with zero `Missing character` reports.

