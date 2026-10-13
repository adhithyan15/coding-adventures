# Script closure — was the reader ever taught these letters? (HL11)

The glyph budget above measures **pace**. It caps how fast new glyphs arrive and
it caught real spikes. But a track satisfies it perfectly while teaching no
letters at all, and that is what most non-Latin tracks do.

```ts
import { loadEverything, measureScriptClosure } from "@coding-adventures/human-language-data";

const { lessons } = loadEverything();
const closure = measureScriptClosure(lessons);

closure.summary.violations;                    // 932
closure.summary.tracksTeachingNothing;         // 12 of 16 non-Latin tracks
closure.summary.headwordsWithoutRomanization;  // 489
closure.summary.exposureExemptedGlyphs;        // 1997 — what the rule removed
closure.unknownScriptTracks;                   // [] — unmeasured, never "clean"
closure.violations[0];                         // { lessonId: "TE-C16-nelalu", count: 30, … }
```

Put the two numbers beside each other and the gap is the argument for the whole
measurement:

| | flagged lessons |
|---|---:|
| script **ramp** — glyphs arriving faster than the budget | **61** |
| script **closure** — glyphs arriving untaught | **932** |

**Exposure is what keeps this from being absurd.** A word the reader is merely
*shown* — the headword printed beside its romanization, so the eye starts
recognising a shape long before the hand can make it — is not something they were
asked to read. HL11 calls that exposure; it is counted, reported, and never
required. That is what lets a Tamil course open on வணக்கம் while still promising
the reader is never asked to decode something untaught.

The distinction has to be mechanical or it is worthless, so it is drawn where the
corpus already records the answer: **a headword is exposure when the lesson
declares a `romanization`.** Which is also why it is the right rule — it names
its own remediation. Adding a romanization converts a headword from load-bearing
to exposure, and that is a real improvement to the lesson rather than a way of
hiding from the measurement. 489 lessons are one romanization away.

Two numbers watch the exemption, not one. `exposureOnly` counts lessons the
rule flipped to clean — 49. `exposureExemptedGlyphs` counts what it actually
removed, including from lessons that violate anyway — **1,997**. The second is
the one that matters: a lesson reporting five untaught glyphs while fifteen more
were exempted is not a lesson with five problems, and a per-lesson count cannot
see that. It is also the number that would move if an author started laundering
script through the headword once 932 becomes a burn-down target.

A glyph counts as **taught** by any `type: writing` or `delivery: script` lesson
that contains it. That is coarser than naming each letter in frontmatter, and
deliberately so: it credits the corpus with everything it could plausibly be
teaching, which makes the reported debt a *lower bound*.
