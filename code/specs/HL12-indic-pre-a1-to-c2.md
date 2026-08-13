# HL12 — The Indic tracks, pre-A1 to C2

**Status:** specification, 2026-08-13
**Applies to:** Tamil, Telugu, Kannada, Malayalam, Hindi, Sanskrit.
**Follows:** HL10, which is this document's counterpart for Spanish. Where HL10
and this agree, HL10 is the original and this is the port.
**Adds what HL10 could not:** Spanish's reader arrives already able to read. These
six have to be taught the script as well, and that changes the shape of the climb.

---

## 1. Where the six actually stand

Measured 2026-08-13, against the committed corpus:

| track | touches | **attained** | vocabulary at or below pre-A1 | script lessons |
|---|---|---|---:|---:|
| tamil | A2 | **none** | 86 | 24 |
| hindi | A2 | **none** | 86 | 11 |
| telugu | A2 | **none** | 79 | **0** |
| kannada | A2 | **none** | 80 | **0** |
| malayalam | A2 | **none** | 84 | **0** |
| sanskrit | A2 | **none** | 53 | **0** |
| *spanish, for scale* | *B2* | *none* | *153* | *n/a* |

Every one of them **points at** A2 and has attained nothing — not pre-A1. HL09
§3.1 sets pre-A1 at roughly 300 words; the best of these has 86.

So the honest starting position is not "these tracks are at A2 and need
extending." It is: **the ladder has not been climbed at all, and the first rung
is four times taller than what exists.**

The owner has confirmed that every handwritten chapter in these tracks is a
**draft**, unpublished and held by nobody else, and may be rewritten. That
removes the constraint that shaped HL11's placement decisions, and it is what
makes this document possible.

---

## 2. The governing idea: two ramps, and only one of them ends

From the owner, and it is the most useful sentence written about this curriculum:

> *"I assume after a sizable number of lessons, reading the script would become
> second nature. But what the script means will become a problem."*

That is exactly right, and it has a consequence the corpus does not yet reflect.
There are **two** difficulties in these books and they behave completely
differently:

| | **decoding** | **meaning** |
|---|---|---|
| the question | what sound is this shape? | what does this word do? |
| how big | finite — an alphabet, its signs, its conjuncts | unbounded — to C2, tens of thousands of words |
| how it ends | **it ends.** It becomes automatic and stops being a topic | it never ends; it is the whole climb |
| when it hurts | at the start | forever after |

A curriculum that treats these as one ramp gets both wrong. It either drags the
script out across the whole book, long after the reader stopped needing it, or it
front-loads the script and makes the reader climb an alphabet before saying a
word — which the owner has already ruled out.

### 2.1 The rule: never steepen both ramps in the same lesson

> **A lesson may sit at the frontier of decoding or at the frontier of meaning.
> Not both.**

This is HL12's counterpart to HL11's closure rule, and it exists because of a
specific failure that is invisible without it. When a reader stumbles on a lesson
that is new in both dimensions, **they cannot tell which one they failed** — and
neither can the curriculum. Did they not recognise the letters, or did they
recognise them and not know the word? Those need opposite remedies: more decoding
drill, or more vocabulary. A lesson that confounds them prescribes neither.

Measured over the six tracks' 577 lessons:

| | lessons |
|---|---:|
| **new glyphs AND new atoms together** | **59** |
| new glyphs only — pure decoding | 86 |
| new atoms only — pure meaning | 330 |
| neither — recap or practice | 102 |

59 lessons ask for both at once. That is the burn-down list, and it is small
today only because four of the six tracks teach no script at all. Authoring the
script ramp without this rule would grow it fast.

The rule is deliberately about the *frontier*, not about total content. A
decoding lesson may use any word the reader already knows — indeed it should,
because a letter is worth what it spells. A meaning lesson may use any letter the
reader already reads. What is forbidden is a lesson whose new letter and new word
arrive together, so that failure is unattributable.

### 2.2 The decoding ladder has an end, and the book says so

Somewhere in every one of these tracks there is a lesson after which the reader
can decode anything the language writes. The book must **name that moment**, and
it must arrive:

```
letters ──► vowel signs ──► the vowel-killer ──► conjuncts and ligatures
   ──► running text ──► speed ──► "you can read anything now"
```

After it, the script is not a topic again. Every later lesson is meaning, and the
reader is told so explicitly, because the alternative is a learner who reads
fluently, understands little, and concludes they are bad at the script.

Everything before that point is **pre-A1 and A1**. Everything after it is the
climb to C2. That is the shape of the book.

### 2.3 Romanization is scaffolding, and it is removed on a schedule

HL11 made a headword *exposure* when its lesson declares a `romanization` — the
reader can use the word without reading it, which is what makes a book about an
unfamiliar script usable from page one. That rule is correct and it has an
expiry date.

**A romanization that never goes away means the reader never has to decode.**
The eye takes the easier path every time; the script stays foreign for a thousand
pages while the reader believes they are learning it.

So romanization is scheduled scaffolding:

| stage | romanization | closure |
|---|---|---|
| pre-A1 | on every headword | exposure exemption applies |
| A1 | on first use of a word only | exemption narrows |
| A2 and above | **absent** | **strict — every glyph must have been taught** |

The three rules compose exactly. HL11's exposure exemption is a pre-A1 device;
as it is withdrawn, headwords become load-bearing, and closure — which was
report-only while the debt was inherited — becomes a gate the track must pass.
The scaffolding comes down as the structure takes the load.

---

## 3. The ladder, where no exam defines one

Four of these six have no widely-sat proficiency ladder, and `core/exam-levels.json`
already records the project's editorial judgement for each with `basis: editorial`.
That file's own rule governs here: an editorial mapping is *a working default to
be corrected, never a claim about what a certificate is worth.*

CEFR stays the backbone because the shared spine already speaks it and because
its descriptors are about what a learner can **do**, which is language-neutral.
Two tracks need a note:

- **Tamil** is diglossic. The written and spoken registers differ enough that a
  single "level" hides which one is meant. Every Tamil rung must say which
  register it certifies; HL09 §8.1's `register` and `variety` fields are where.
- **Sanskrit** is taught here as a living spoken language, so CEFR's descriptors
  apply. A traditional syllabus is ordered by grammatical topic instead, and the
  two do not line up; where they conflict, the can-do descriptor wins, because
  this book is for a learner who wants to use the language.

### 3.1 Size is not a constraint, and must not be treated as one

HL09 §3 puts a complete track at roughly 8,000 lessons and Spanish at 146 when it
was written. Six tracks to C2 is therefore on the order of **48,000 lessons**,
plus a bounded script ramp of perhaps 150-250 lessons each.

Those numbers are context, **not a budget to work against.** The owner's
instruction is explicit, and worth quoting because it is the opposite of the
instinct an author brings to it:

> *"Don't worry about that. Write as many chapters as you need. Even if the book
> is 10000 pages long, it is fine. We can chop it up into pre-A1, A1...C2 in the
> future."*

So: **no rule in this curriculum may be relaxed to save pages, and no lesson may
be merged with another to shorten a chapter.** Where a lesson is too big, it
splits. Where a chapter needs forty lessons to stay gentle, it gets forty. HL08
already forbids any threshold that penalises page, lesson or chapter count; this
extends that from the gates to the authoring, because a gate that permits length
does not help if the author is quietly economising anyway.

The failure this prevents is specific, and it is the one that produced the corpus
being fixed here. `HI-W01-shirorekha-na-ma` teaches twelve Devanagari glyphs in a
single lesson. Nothing forced that. It is what an author does when they feel a
chapter should not run long. Twelve gentle lessons would have been correct and
would have cost eleven more pages, which is nothing.

**One book per track, however long.** The split into pre-A1 ... C2 is a later
filter over the same source, not a reason to shape the source now -- the same
one-curriculum-many-editions rule HL08 established for the driving edition.

What this document fixes is the *shape*. Filling it is many tranches of work, in
lockstep across the six.

---

## 4. What each level means here

Level content follows HL10's eight strands. Two are shaped differently for these
tracks and are specified here.

### 4.1 SCRIPT is a strand that retires

Unique among the strands: it is complete at A1 and silent thereafter. Its rungs:

| stage | the reader can |
|---|---|
| pre-A1 | recognise and write the letters their known words are built from |
| pre-A1 | attach the vowel signs; use the vowel-killer |
| A1 | read conjuncts and ligatures without decomposing them consciously |
| A1 | read running text at a speed that does not interrupt comprehension |
| A1 | **read anything the language writes** — the strand closes |

The last rung is the one that must be tested rather than assumed. Reading
*anything* means unfamiliar words, and that is exactly the skill the reader needs
for every level above: **decoding a word you do not know is how you look it up.**

### 4.2 LEXICON carries the weight after A1

Once the script retires, essentially all remaining difficulty is meaning.
HL09's cumulative vocabulary targets are the spine of A1 → C2, and the tracks are
at 53–86 against pre-A1's 300.

This is where the owner's warning bites. A reader at the end of the script ramp
can *read* a page of Tamil aloud and understand almost none of it. **The book must
say so, at that exact point**, and reframe what comes next — otherwise fluent
decoding with no comprehension reads as failure rather than as the expected
halfway house it is.

---

## 5. Rewriting the drafts

Every handwritten chapter in these six tracks is a draft. They are rewritten into
the generated pipeline, which resolves three problems at once, all found while
implementing HL11:

1. **Content that never reaches the page.** All 11 Hindi writing lessons render
   only in the answer key, because they sit in protected handwritten chapters —
   so the Hindi book teaches no writing at all, while its chapter 1 prose
   promises *"Each lesson introduces the letters its word needs."*
2. **Order that lives only in LaTeX.** Five tracks carried ~30 lessons with no
   `sequence`; the order existed solely in hand-typed `.tex`. Recovered in
   HL11's ordering pass, but the underlying hazard is the split itself.
3. **Placement forced by protection.** Tamil's drizzle sits later than it should
   because chapters 1–5 could not be touched.

The prose in those drafts is good and is **carried across, not discarded**. What
changes is that it becomes lesson data with a declared order, a chapter
capability, and a place in the measurements — rather than LaTeX that only a human
reads.

---

## 6. What gets measured

Everything ships report-only first, per the HL05/HL08 precedent, and becomes a
gate per track as that track is rebuilt.

| measurement | catches | target |
|---|---|---|
| **both-ramps-steep lessons** | a lesson whose failure is unattributable | 0 |
| decoding-ladder completion | the script strand never closing | one named lesson per track |
| romanization schedule | scaffolding that never comes down | absent at A2+ |
| closure (HL11) | glyphs asked for but never taught | 0, and gating from A2 |
| cumulative vocabulary | the real distance to each rung | HL09 §3 targets |
| level attainment (HL09 §3.1) | claiming a rung by touching a node | pre-A1 first, honestly |

The first is new and is this document's contribution. The rest exist and are
listed so a reader can see the whole instrument panel in one place.

---

## 7. Out of scope

- **Which letters, in what order.** That is the per-script letter ledger (HL11
  §4), authored per script and reviewed on its own.
- **Stroke-order sourcing.** HL11 §5: shape verified against the font, order
  cited or absent. Four of these five scripts have no cited ductus yet.
- **The figure pipeline.** The step-by-step writing bitmaps are specified in
  HL11 §6 and are independent of this ladder.
- **Deciding any track's level rungs in this document.** Each is authored per
  track against the measurements above, and reviewed on its own.

---

## 8. Provenance

| claim | source |
|---|---|
| attainment, touches, vocabulary per track | `runLevelGate` over the committed corpus, 2026-08-13 |
| 59 lessons steep in both ramps | walk of the six tracks in `sequence` order, 2026-08-13 |
| script-lesson counts | `measureScriptClosure`, same date |
| ~300 words at pre-A1, ~8,000 lessons to C2 | HL09 §3, carried forward unchanged |
| editorial CEFR mappings and their caveats | `core/exam-levels.json` |
| handwritten chapters are drafts | owner, 2026-08-13 |
| the two-ramp framing | owner, 2026-08-13, quoted in §2 |
