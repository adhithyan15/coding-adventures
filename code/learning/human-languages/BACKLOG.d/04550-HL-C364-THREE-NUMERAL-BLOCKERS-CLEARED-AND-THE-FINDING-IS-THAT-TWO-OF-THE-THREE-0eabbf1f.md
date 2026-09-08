## HL-C364 — Three numeral blockers cleared, and the finding is that two of the three were priced wrong by their own notes

**MEASURED WITH HL-C350'S OWN CHECKED-IN SCRIPT, RE-RUN AGAINST `origin/main`
BEFORE THIS WORK AND AGAIN AFTER EACH TRANCHE.** Re-run it rather than trusting
these rows — HL-C358 and HL-C360 both recorded per-track figures that had aged by
the time the next reader arrived:
`node code/learning/human-languages/data/scripts/numeral-probe.mjs`.

**WHAT MOVED.** Three points closed on three tracks, in three separate PRs:

- **`GU-A1-NUM-03`** — Gujarati cardinals six to ten. 121/210 → 122/210. Eight
  lessons, seven atoms, **one** new letter.
- **`BN-A1-Q-03` AND `BN-A1-LIP-11`** — the Bengali digits, TWO points from one
  tranche because they are the same work filed in two columns and the LIP-11 note
  said so itself. 139/244 → 141/244. Twelve lessons, eleven atoms, ten glyphs.
- **`JA-A1-NUM-02`** — the Japanese counters, and the first `japaneseSpecific`
  point in that file to close at all. 66/179 → 67/179. Twenty-six lessons in
  three chapters, twenty-three atoms, ten glyphs.

**THE FINDING THIS ENTRY ADDS: A NOTE THAT PRICES A POINT IS WRITTEN BY SOMEBODY
LOOKING AT THE POINT, AND TWO OF THESE THREE WERE PRICED WRONG BY THEIR OWN.**
HL-C359 established that for absence — Marathi's "Untaught" ordinal was taught in
chapter 31. This round found the same failure for **cost**, in both directions:

- **The Japanese counters were priced at "4+ new kana" and the corpus said
  FIVE.** Deriving the taught set at point of use rather than transcribing it
  turned up **の**, which `ここのつ` needs and which is easy to miss because
  *kokono-* looks like a compound and is not one. A count derived from the corpus
  is the only kind worth having; this is HL-C359's hand-copied-census lesson one
  level up, applied to a *forecast* rather than to a completed tranche.
- **The Gujarati cardinals were priced as ordinary vocabulary and cost almost
  nothing.** Four of the five numbers need no new sign at all: **છ**, six, is a
  single consonant, and it is the letter the reader has been writing inside
  **છે** since chapter two. Only **આઠ** needed a letter. A note that says "the
  count stops at five" does not say how much reaching ten costs, and the two
  questions have different answers.

**THE SECOND FINDING IS ABOUT ORDER, AND IT CORRECTS A HABIT RATHER THAN A
FACT.** HL-C354 said order by the language's own construction, or by cost when
there is none. Cost-ordering has been the default for numerals ever since, and it
produced the hole at six that the Japanese cardinal chapter's own note records —
three lessons of counting round a gap. **Cost order is right only when the gap
buys something.**

- **Gujarati** ran in COUNTING order, because Wiktionary's `-મું` entry ends its
  exception list at **છ**: the reader crosses the boundary between exception and
  rule exactly once, and in counting order that crossing falls between the first
  lesson and the second, where it is legible. Cost order would have moved **આઠ**
  to the end and bought nothing.
- **The Japanese native series** ran in cost order but bought each of its five
  signs in the lesson **immediately before** the word that needs it, so the count
  grew contiguously and there was no hole at all. That is a third option the
  earlier chapters did not use, and it is available whenever the exception the
  hole was protecting does not exist.

**THE THIRD FINDING IS A CLASS OF SILENT ERROR THAT ONLY A PER-LESSON WALK SEES.**
A script lesson teaches every target-script glyph in its body, so a glyph quoted
in a script lesson is marked TAUGHT — a false claim the aggregate cannot show,
because closure and never-taught both stay at zero while the corpus now believes
a letter was taught. A per-lesson walk over each new chapter, with the taught set
derived from the corpus at point of use, caught three kinds:

- **Nine kanji entering a Japanese chapter through Wiktionary LINK LABELS.** This
  is the same shape as the Punjabi finding HL-C359 recorded — *a URL is not prose
  but a link label is* — and it is worse in a script lesson, because there the
  leak does not merely violate closure, it silently claims the glyph is taught.
- **Two Japanese kana in a passing example**, and **a digit appearing one lesson
  before its own lesson** in Bengali.
- **A Bengali letter entering through two clothing words in warm-up recalls**,
  which were rewritten to cue their words in romanization: the same lexical atom
  is retrieved and no letter is taught by accident.

**FOURTH, AND IT IS THE ONE THAT CHANGED A CHAPTER'S SHAPE.** The Japanese native
count was authored as ONE chapter of fifteen atoms. `maxNewAtomsPerChapter` is
**12**, and the gentle-ramp atom-step finding for Japanese went from zero to one —
on the only track in the corpus that has never carried a finding of any kind. The
fix was not to raise the budget: the chapter was split at five, exactly where
chapters 14 and 15 split the borrowed count, giving 7 and 8 atoms. **A budget that
forces a split at the place the track already splits is a budget that is right.**

**WHAT THE COVERAGE TOTALS CANNOT SEE, PER TRACK.** All three tranches moved one
or two points, and in each case the total is the least of what changed:

- **gujarati** — the probe's highest reachable numeral goes **5 → 10** and its
  numeral lessons **1 → 6**; the `-મું` ordinal rule stops being a single worked
  example and becomes productive without bound, so `GU-A1-NUM-05`'s "not claimed"
  note is discharged without a lesson.
- **bengali** — before chapter 39 the track had **sixty script lessons and not
  one of them put a non-letter on the page**. No price, date, clock face or page
  number was readable at all. `taughtGlyphs` 37 → 47, with violations and
  never-taught both unchanged, because a digit was never *shown* before it was
  taught.
- **japanese** — the track's SECOND counting system now exists, and a number can
  be attached to a noun for the first time in 157 lessons. `JA-A1-NUM-03` changes
  from "blocked upstream" to ordinary vocabulary work.

**REINFORCEMENT, ACROSS ALL THREE, AND THE JAPANESE FIGURE IS THE ONE TO KEEP.**
Gujarati 362 → **360**, Bengali 242 → **242** (every window flat), Japanese
**0 → 0 in every window**. Twenty-six Japanese lessons made **47** (atom, window)
slots newly judgeable and answered all of them, on a track that had no miss
anywhere to begin with. The method was arithmetic rather than search in all
three: an atom introduced at position *p* has its fourth window open at *p*+80,
so new lesson *n* services the atom introduced at *n*−80, one per lesson. In
Japanese that lined up exactly — twenty-six lessons, twenty-six R4 slots — and
**the diagonal was departed from only where a better pairing existed**: `いちど`'s
R4 was moved to the lesson that first says the word *counter* out loud, and four
body words landed in the four lessons where a reader would actually be counting
them.

**STROKE ORDER, AND THE TWO DIFFERENT ANSWERS IT GOT.**

- **Japanese**: six new kana, every one **observed**. Sirgazil's CC0 Commons
  animations were downloaded and read frame by frame, and cross-checked against
  KanjiVG's path data for the same codepoint. **や** is the one that justifies the
  procedure: its finished shape says the long descender is drawn first, and both
  sources agree it is drawn **last**.
- **Bengali**: **no pen path is claimed for any of the ten digits**, and that was
  searched for rather than assumed. HL-C212 found Commons holds animations for
  every Bengali vowel and no consonant; the same search for the digits finds
  photographs and no writing guidance, and the letter-writing app the Gujarati
  track cites covers Bengali letters only. Every digit lesson therefore teaches
  the SHAPE against something the reader already holds and cites the Unicode
  chart, exactly as `BN-W05-tha` and `BN-W06-nya` do.
- **Gujarati**: one letter, **ઠ**, observed from the animation the track's other
  29 writing lessons already cite — one unbroken segment of 50 points, and the
  same read gives the discrimination against **ટ**, which turns the other way and
  finishes open.

**SHAPE CLAIMS WERE RASTERISED, NOT REMEMBERED.** The Bengali chapter's whole
order rests on which digits look like which Western ones, so the ten digits were
rendered in the book's own vendored font at 80pt and looked at. **৪ is drawn as
an 8 and means four; ৭ is drawn as a 9 and means seven**; and **৩** differs from
the taught letter **ও** by one right-hand stroke. Those are the three claims the
chapter is built on and all three were read off the page.

**WHAT IS STILL OPEN IN THIS COLUMN.**

- **`GU-A1-NUM-08`, the Gujarati digits, zero of ten.** The cardinals closing did
  not touch them, and the note now says why: the digits are a script problem, not
  a vocabulary one, and they are the column's only remaining reading gap below
  ten.
- **`BN-A1-Q-01`, the Bengali cardinals past five.** The digit chapter sharpened
  this rather than shrinking it — the reader can now READ every number and SAY
  only the first five — and two of the missing words additionally need **য়**, a
  letter the track does not teach.
- **`JA-A1-NUM-03`, the Japanese ordinals**, now unblocked and ordinary.
- **`marwadi MW-A1-Q-02`** remains the column's one genuine SOURCING block, per
  HL-C358 and HL-C359. Do not invent one.
- **`italian IT-A1-ORT-16`**, the superscript ordinal indicator, is a typography
  point and should still not be counted with the vocabulary ones.
