# HL11 — The drizzled script ramp

**Status:** specification, 2026-08-12
**Applies to:** every track whose script the English-reading learner does not already
know. Authored against the six Indic tracks — Tamil, Telugu, Kannada, Malayalam,
Hindi, Sanskrit — and language-neutral by construction.
**Builds on:** HL08 (§ the script ramp, `maxNewGlyphsPerLesson`, modality),
HL09 (§4 the lesson contract, §4.1 no forward references), HL10 (§ cells, the
Root Ledger, "measure before building").
**Does not replace:** any of them. It adds the one ramp none of them can express.

---

## 1. Why this exists: two ways to get the script wrong, and the corpus has both

HL08 already counts new target-script glyphs per lesson and caps them at three. That
budget is real and it caught real spikes. But a budget on *how fast* glyphs arrive
says nothing about *whether the reader was ever taught them*, and the six Indic
tracks fail on exactly that axis — in two opposite directions at once.

Measured on the committed corpus, 2026-08-12, walking each track's lessons in
`sequence` order:

| track | lessons | on page 1 | by lesson 10 | by lesson 50 | writing lessons | first writing lesson at |
|---|---|---|---|---|---|---|
| tamil | 132 | 4 letters + 1 mark | 15 + 8 | 38 + 11 | 20 | `sequence: 270` |
| telugu | 87 | 4 + 4 | 38 + 11 | 49 + 12 | **0** | — |
| kannada | 88 | 7 + 5 | 39 + 11 | 52 + 12 | **0** | — |
| malayalam | 93 | 9 + 5 | 44 + 12 | 52 + 13 | **0** | — |
| hindi | 109 | 7 + 5 | 21 + 7 | 41 + 14 | 11 | lesson 1 |
| sanskrit | 59 | **12 + 8** | 24 + 11 | 35 + 13 | **0** | — |

Read the two ends of that table together.

**Wrong in one direction — script the reader cannot decode, from page 1.**
`TA-C01-vanakkam` sits at `sequence: 10` and prints வணக்கம். That is four Tamil
letters and the puḷḷi, and not one of them has been taught, because Tamil's writing
lessons begin at `sequence: 270`. The reader meets Tamil script on page 1 and gets no
path to reading it for a hundred pages. `SA-C06-numbers-1-5` opens **twenty** distinct
Devanagari glyphs in a single lesson. Telugu, Kannada, Malayalam and Sanskrit have no
writing lessons at all, so for those tracks the number of taught letters is not late.
It is zero, forever.

**Wrong in the other direction — the obvious fix, which is worse.** Put the script
first: an alphabet course up front, then the language. This is what most script-heavy
courses do, and the project owner has explicitly rejected it:

> The books have to be useful from page 1. So, do not start with script first.
> Instead, you can slowly ramp up on that. For example, teach greetings first and
> then slowly drizzle in one letter at a time. By the time we get to Lesson 50 or
> something like that we could have the readers be able to write a few words.

That is correct and it is not a compromise. A reader who spends fifty pages on
அ ஆ இ ஈ before saying a single word has been handed a chore, not a gentle ramp. HL09
§4 already requires that every lesson pay off immediately; a front-loaded alphabet
violates it fifty times in a row.

So the requirement is both things at once: **useful from page 1, and never asked to
read something untaught.** Those look contradictory. §2 shows why they are not.

### 1.1 A third finding, which explains why "lesson 1" above is sometimes nonsense

Thirty to thirty-four lessons in each of Telugu, Kannada, Malayalam, Hindi and
Sanskrit carry **no `sequence` at all**. Their reading order exists only in
hand-typed LaTeX. That is why the walk above reports Telugu's first lesson as
`TE-C06-dative-ku` — the dative case — and Malayalam's as `ML-C06-dative-ikku`.
HL09 §4 already makes `sequence` mandatory and HL09's continuity report already
counts its absence corpus-wide; HL11 simply records that **a script ramp cannot be
verified at all until it is fixed**, because every claim in this spec is a claim
about order.

---

## 2. The design: two ramps, side by side

The contradiction in §1 dissolves once you notice that "knowing a word" and "reading
a word" are different acquisitions with different prerequisites, and the corpus
already carries both channels.

### 2.1 The speaking ramp — from page 1, unblocked by the script

Greetings, names, how-are-you, thanks. The learner meets each word through its
**romanization**, a first-class field the schema already requires for non-Latin
tracks and which the narration export and PDF bookmark generator already depend on.
A word delivered this way needs no letters. It is `voice` modality, so it is
drivable, and HL08's machinery already delivers it.

`TA-C01-vanakkam` stays at `sequence: 10`. It stays the first lesson. It stays
useful. Nothing about the opening of the book gets worse.

### 2.2 The script ramp — one letter at a time, running behind it

A short **script segment** appears in roughly every second or third early lesson and
teaches exactly **one** letter: what it looks like, what it sounds like, where it has
already been seen, and how the hand forms it. Twenty letters by lesson fifty is a
gentle rate by any measure — and §3 shows twenty is far more than enough.

### 2.3 The rule that joins them: closure on *load-bearing* script only

> **A lesson may ask the reader to decode or produce target-script text only when
> every glyph in that text has already been taught.**

Target-script text the reader is merely *shown* — the headword displayed beside its
romanization, so the eye starts to recognise a shape long before the hand can make
it — is **exposure**. Exposure is never decoded, never drilled, never required, and
never charged against the budget.

That single distinction is what lets வணக்கம் sit on page 1 while the reader is still
never asked to read something untaught. It is also honest about what page 1 actually
is: the reader is being *shown* a word, the way a child sees a shop sign years before
reading it.

The two are distinguished in the lesson, not guessed at by a validator:

| in the lesson | means | closure applies? | budget charged? |
|---|---|---|---|
| headword display beside its romanization | exposure | no | no |
| a `script` block teaching a letter | **teaches** the glyph | n/a — this is the source | one letter |
| reading practice, a drill, an activity answer, a writing segment | **load-bearing** | **yes** | yes |
| a cousin-script comparison table | context (HL08) | no | no — already exempt |

The fourth row is not new. HL08 established that a Kannada chapter showing the same
word in four sister scripts has a Kannada load of 7, not 34, and that foreign glyphs
are reported and never charged. Exposure is the same principle applied within the
target script: **counted, reported, never charged, and never required.**

### 2.4 Where the handwriting goes, so the driving edition survives

Each letter's handwriting portion is authored as a **detachable `writing` segment**
inside an otherwise-`voice` lesson. HL-C41 built precisely this and nothing in the
corpus has used it yet: `coreModality` is the lesson minus its detachable blocks,
`writing` is the one detachable block type, and `drivablePercent` is counted on the
core, not the whole.

So the greeting course stays drivable end to end while the pen work rides along in
the printed book. **Detachable never means optional** — the book prints every block
in full. It means a non-visual renderer may set it aside.

This gives a hard, falsifiable check on the whole design, stated in §7: when the
drizzle lands, `drivablePercent` must not fall.

---

## 3. Measure first: how many letters is "a few words"?

HL10's discipline is that a cheap measurement often redefines the task. Here it does.

Before deciding how fast to drizzle, ask the corpus: taking each track's existing
target-script headwords, and choosing letters greedily by how many *whole words* each
one completes, how many glyphs does it take before the reader can write a real word?

Measured 2026-08-12 over every distinct target-script headword in each track:

| track | distinct headwords | glyphs to the **first** writable word | glyphs to **five** writable words |
|---|---|---|---|
| tamil | 113 | 5 | 8 |
| telugu | 80 | 2 | 7 |
| kannada | 81 | 3 | 8 |
| malayalam | 87 | 3 | 7 |
| hindi | 100 | 1 | 7 |
| sanskrit | 55 | 1 | 8 |

**Seven or eight glyphs unlock five real words, in every one of the six tracks.**

That is the number that makes the owner's instruction easy rather than ambitious. At
one letter per script segment and a segment in every second or third lesson, eight
glyphs land somewhere around lesson twenty, and lesson fifty is not a stretch target —
it is comfortable. The drizzle can be slower than the ceiling, which is what makes it
gentle.

It also settles the ordering question in §4, because the number depends entirely on
*which* letters. The same measurement run in traditional recitation order — twelve
vowels first — completes **zero** words after twelve glyphs.

> **Caveat, stated because the number is quotable.** This greedy walk runs over the
> *existing* headword set, which is itself being rewritten. It is evidence that the
> approach works and roughly how fast, not the final ledger. §4 says how the real
> ledger is produced.

---

## 4. Letter order is chosen by word payoff, and recorded as data

The traditional order of every one of these scripts is a *recitation* order —
அ ஆ இ ஈ உ ஊ, अ आ इ ई उ ऊ — organised by phonology, for a learner who already
speaks the language and is learning to write it. It front-loads independent vowels,
which in an abugida appear in relatively few words, because the vowel that actually
does the work in running text is the *sign* on a consonant.

This curriculum's reader is the opposite person: they cannot yet speak, and the
letters are worth exactly what they unlock. So:

> **Letters are ordered by the words they make writable, not by recitation order.**

Each script gets a **letter ledger**: an ordered list, one entry per letter or vowel
sign, declaring the position, the words that letter completes, and the lesson that
teaches it. The ledger is *authored intent* in the sense `chapters.json` is — a
validator may check it and may never rewrite it.

This is the Root Ledger's rule (HL10) applied to glyphs. A root must be cashed in by
at least three later lessons or it is cut or moved; a letter taught early that
completes no word for twenty lessons is the same waste, and the ledger makes it
visible.

Two constraints the payoff ordering does not get to override:

1. **Teach families together where the shapes are confusable.** Tamil ண/ன/ந/ற share
   a flat top bar and are already documented as best learned as a family; splitting
   them across twenty lessons to chase payoff would trade a reading ramp for a
   writing confusion.
2. **One writing system at a time**, unchanged from HL08's
   `maxNewScriptSystemsPerLesson: 1`. Hindi and Sanskrit share Devanagari; a learner
   doing both is not doing two scripts, and a learner doing Tamil and Telugu together
   is, and may not meet both in one lesson.

### 4.1 A letter is taught in three stages, not one

Gentleness inside a letter matters as much as gentleness between letters. Each letter
crosses the ramp three times, in separate lessons:

| stage | the reader | modality | closure |
|---|---|---|---|
| **see** | recognises the shape, in a word already known by ear | `sight` | introduces the glyph |
| **read** | decodes it inside a word whose other glyphs are taught | `sight` | requires closure |
| **write** | forms it by hand, from the step-by-step figure | `pen`, detachable | requires closure |

Only the **write** stage needs a pen path, and therefore only the write stage can be
blocked by §5's citation rule. A letter with no citable stroke order still completes
*see* and *read*, and the reader can still read every word it appears in. That is
what keeps a sourcing gap from stalling a chapter.

---

## 5. The ductus contract: what may be claimed about how a letter is written

The finished shape of a letter is knowable from the font we ship. **The order the
pen travels is not recorded anywhere in a font**, and no authoritative
machine-readable stroke database exists for any Indic script. So the shape is
verified and the order is cited, and the two are never confused.

### 5.1 Shape: verified against the font, never believed

A pen path is authored by hand and then checked, by the discipline already in the
ductus tests:

1. every point of a stroke's path lands on the real glyph's ink (`fractionOnInk`);
2. within a stroke, consecutive segments **meet** — the end of one part is the start
   of the next — so "these parts do not force a lift" is proved, not asserted;
3. the strokes together pass near **all** of the letter's ink, so the path traces the
   whole letter and not just the easy half.

That converts "did the author draw this correctly", which no reader could check, into
"does this path lie on the font shape", which a machine checks exactly.

### 5.2 Order: cited, or absent

`strokeOrderSource` carries `citation`, `url`, and `variation` — the last recording
how much the attested teaching order varies, because for these scripts it varies a
great deal and a spec that hid that would be lying. The schema already refuses
`penLifts` without a `strokeOrderSource`; HL11 extends the same refusal to the pen
path itself.

> **No citation → no pen path → no figure.**

The letter still ships with prose `components`, a *see* stage and a *read* stage. The
missing figure is reported as measurable debt, not silently absent. This is the
project's standing preference for a stated gap over an invented fact, and it is the
owner's explicit instruction for this work.

Where a source is found but is one school's practice rather than a standard, that is
what `variation` is for. Tamil's existing eleven letters are the template: cited per
frame to Radhakrishnan's *Tamil Script Learners Manual*, with the variation note
saying plainly that Tamil handwriting is taught with school-to-school variation and
there is no national standard.

### 5.3 Scope: base letters and signs, not the composed grid

Telugu, Kannada and Malayalam each carry 455–468 generated syllabary rows. Ductus is
authored for **base letters and vowel signs only**; a composed syllable is the base
plus the sign, and its figure is derived from theirs. Authoring 455 pen paths per
script would be authoring the same forty movements eleven times.

---

## 6. The figure: a filmstrip, one panel per stroke

Each writable letter gets a **step-by-step raster figure** — the artifact the owner
asked for, and the only thing in the book that can actually teach a hand to move.

One letter renders to *n* panels, where *n* is the number of pen-down strokes. Panel
*k* shows:

- the **finished glyph outline**, behind, in pale grey — read from the shipped font,
  never a drawing of one, so the target and the ink can never disagree;
- the pen path **as far as stroke *k***, in ink;
- a **dot** where the pen is at that instant;
- a **caption** naming the movement, taken from the segment labels.

Rendered as **PNG**, because the book is XeLaTeX and XeLaTeX embeds PNG. Generated,
never hand-drawn; declared in the figure-generation config; and **byte-gated** by the
generated-figure hash manifest exactly as the one existing generated figure is, so a
stale claim and an edited artifact both fail `--check` rather than shipping a picture
that no longer matches the letter.

The same description already drives the app's on-screen filmstrip. One source, two
consumers — which is the standing rule that the book and the app may not drift.

---

## 7. What gets measured, and what "gentle" now means

All of it ships **report-only** first, per the HL05 and HL08 precedent: the debt in
§1 predates the measurement, and a gate that fails on recorded debt teaches authors to
route around it. Each number is enforced per track as that track is rebuilt.

| measurement | what it catches | target |
|---|---|---|
| **closure violations** | load-bearing script the reader was never taught | 0 per track |
| **`firstWritableWord`** | the sequence at which a real taught word becomes writable | ≤ ~50 |
| **writable-word curve** | writable words as a function of sequence | rises early, keeps rising |
| **letters per script segment** | the drizzle rate | exactly 1 |
| **unspent letters** | a letter taught early that completes nothing for a long time | 0 |
| **uncited letters** | letters with prose order but no citable source | falls; never hidden |
| **`drivablePercent`** | whether the pen work broke the car edition | **does not fall** |

The last row is the design's own falsification test, and it is worth stating as a
prediction rather than a hope: if the writing segments are genuinely detachable, the
share of the corpus learnable by ear cannot drop when they land. If it drops, the
segments were authored wrong, and the number says so before a reader does.

`firstWritableWord` deserves one note on why it is a *word* and not a letter count.
Twenty taught letters is not an achievement the reader can feel. Writing their own
name, or *thank you*, is. The measurement is deliberately of the payoff, not of the
effort — the same reason HL05 measures a chapter by what the reader can do at the end
of it.

---

## 8. What this does to the existing six tracks

Nothing is bulldozed. The owner's instruction is to keep what works and move what does
not, and most of it works:

- **Tamil's twenty `TA-W*` writing lessons are good material in the wrong place.**
  They are re-cut into one-letter segments and spread across the early sequences
  instead of clustering at 270. The prose largely survives; the packaging does not.
- **Hindi's eleven `HI-W*` lessons** likewise. `HI-W01-shirorekha-na-ma` opens twelve
  Devanagari glyphs in one lesson and is the steepest lesson in the entire corpus; it
  becomes a dozen drizzled segments.
- **Word lessons keep their content and their usefulness.** What changes is that their
  native-script text is marked as exposure until closure catches up with it.
- **Telugu, Kannada, Malayalam and Sanskrit have no script strand to move**, so theirs
  is authored from nothing, in letter-ledger order.
- **233 of the six tracks' 568 lessons are schema-v1** and are therefore invisible to
  every gate in this spec. They are migrated to v2 as each track is rebuilt; until
  then, every number here is a lower bound on the real debt, and is reported as such.

Length is never traded for any of this. Where the ramp needs more room, lessons are
**added**, per HL08's rule that no threshold may penalise page, lesson or chapter
count.

---

## 9. Out of scope

- **Audio.** As HL04 and HL08 already state: no TTS, no recordings. The narration
  export is a script *for* a voice agent.
- **Handwriting recognition.** The book and app show the reader how to form a letter.
  Neither judges what they drew.
- **Cursive, calligraphic and historical hands.** One contemporary printed-style
  ductus per letter, matching the shipped font.
- **The composed syllabary grid as authored data** (§5.3).
- **Deciding a track's letter ledger in this document.** Each ledger is authored per
  script, against the payoff measurement, and reviewed on its own.

---

## 10. Provenance of every number in this document

| claim | source |
|---|---|
| lesson counts, schema-v1 counts, writing-lesson counts | committed corpus, 2026-08-12 |
| glyphs on page 1 / by lesson 10 / by lesson 50 | walk of each track's lessons in `sequence` order, 2026-08-12 |
| glyphs to the first and fifth writable word | greedy payoff walk over each track's distinct target-script headwords, 2026-08-12 |
| lessons without `sequence` | same walk; corroborates HL09's continuity report |
| 20 verified pen paths / 208 uncited entries across ten scripts | `BACKLOG.md`, HL-C19 |
| Tamil's cited stroke orders | Radhakrishnan, *Tamil Script Learners Manual*, Appendix I, Univ. of Texas at Austin |

The greedy payoff walk is an editorial planning instrument, not a claim about how any
language is taught by anyone else. It is recorded here so a later reader can see
exactly which question produced the number seven.
