## HL-C282 — Telugu chapters 1-5 are generated, and the schema-v2 migration is what unblocked it

Telugu now has **no hand-written chapters**. The five that remained were moved
from `book-generation.d/handwritten.d/` into `targets.d/` and are built from
their lessons like every other chapter, taking the corpus's handwritten holding
from **69 chapters to 64**.

Three things are worth recording, because each was a surprise.

### The blocker was not the prose. It was schema v1.

`handwritten_parity.py` measures what a flip would DELETE, and it put Telugu at
7 blocks. Carrying those was an afternoon. The actual wall was one line in
`book.ts`:

    TE-C01-namaskaram: generated books require schema version 2

All thirty word, phrase and recap lessons in chapters 1-5 were schema v1 — the
same thirty the gentle-ramp report had been calling `measurement-blind` for
months. **Retiring a hand-written chapter and migrating its lessons to schema v2
are the same task**, and the parity script cannot see that half of it. Any track
whose early chapters are still v1 — kannada is the next one — should be planned
as a migration, not as a flip.

### Migrating to v2 is what made the chapter payoffs measurable

A schema-v1 chapter has no typed atoms, so `payoff.assesses` was `[]` and
`chapter-payoff-not-representative` could not fire — the chapter looked clean by
being unmeasurable. With atoms in place, chapters 1-5 now declare payoffs that
assess 0.83-1.00 of what each chapter introduces, and **telugu joins the
zero-chapter-debt list** in `chapters.test.ts` for the first time.

The same migration made two report numbers move in the OTHER direction, and
both are honest:

* `ramp: … chapters above 12` gains five. Chapters 1-5 introduce 17-28 atoms
  each — but they are 9-15 lessons long, because the letter ladder lives in
  them. The per-LESSON budget is met everywhere (nothing above 3). The chapter
  budget of 12 was calibrated on 4-6 lesson chapters, and a chapter carrying a
  script strand cannot meet it without being split. **Splitting chapters 1-5 is
  the real fix and it is not done here**: chapter numbers are load-bearing in
  `chapters.d`, the book-generation ledger, and the language-ladder's hardcoded
  counts, so it wants its own tranche.
* `script closure` gains three Telugu violations, 6 → 9. See below.

### Atom SLICING moves the ramp number, so the slicing has to be defensible

The first pass gave each word lesson a `TE-SCRIPT-C0n-WORD` atom for its "The
letters in this word" section, alongside LEX, ETYMON and whichever of
GRAMMAR/PRAGMATICS it had. Four atoms for a lesson that teaches one word — and
fourteen lessons went over `maxNewAtomsPerLesson: 3` **without a single lesson
changing**. The author had simply sliced finer.

That is a general hazard: `measureRamp` counts declared atoms, so a migration
can manufacture ramp debt out of nothing, or hide it, purely through
granularity. The rule adopted here is that **reading a word off the page is part
of knowing the word, not a second atom** — the letter strand already owns letters
one at a time as `TE-SCRIPT-RECOG-nn` — so LEX is introduced by the block that
first shows the word, and the corpus figure went back to its baseline of 40.

### Two follow-ups this left on the floor

**The renderer strips ZWNJ out of the script run.** `TE-C05-undu` spelled
హైదరాబాద్‌లో with U+200C, and the generated LaTeX came out as
`\te{…ద్}‌\te{లో}` — the joiner OUTSIDE both script groups, where it cannot do
its job and where Latin Modern has no glyph for it, so `glyph-coverage` failed.
The character was dropped from the lesson, which is right for this word, but the
emitter should carry format controls (U+200C, U+200D) inside the adjacent script
run rather than emitting them as main-font text. No other track has hit this yet
because no other track's generated books contain one.

**Five characters are the next ladder rungs.** The three new closure violations
are the chapter recap tables for chapters 3-5, which show the chapter's sentences
in Telugu. Those tables were in the hand-written `.tex` all along; moving them
into the lessons is what made the debt visible, and the exemption is why the word
lessons themselves never showed it (a headword with a romanization is exempt, and
the exemption subtracts those glyphs from the body too). The untaught characters
are **గ · బ · ళ · ె · చ**. Teaching them is letter-ladder work and belongs with
whoever owns that chain, not with a migration tranche.
