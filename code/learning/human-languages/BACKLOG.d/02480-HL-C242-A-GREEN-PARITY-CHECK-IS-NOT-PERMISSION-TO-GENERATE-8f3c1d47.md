## HL-C242 — a green parity check is not permission to generate

Sanskrit's hand-written chapters 1–5 are retired. `handwritten_parity.py`
reported the track under NOTHING WOULD BE LOST and `--check sanskrit` exited 0
*before any work was done*, which made this look like a five-file flip. It was
not, and the three things that made it not are the reusable part.

### 1. The parity script cannot see a block that belongs to no lesson

Chapter 2's hand-written closing section, "The whole exchange," carried
`\label{SA-C02-practice}` — **a lesson id that did not exist**. Chapter 2 was
the only one of the five without a recap lesson. Parity compares a chapter's
prose blocks against what its lessons would produce; a block whose owning lesson
is absent from the corpus is not a block that *moves*, so nothing registered as
lost. Generating the chapter as it stood would have silently dropped a
four-line introduction dialogue and its closing paragraph.

The check is doing exactly what it says: it measures whether a prose BLOCK
disappears. It cannot measure whether the chapter still teaches the same thing.
**The side-by-side read is the gate; parity is a pre-filter.** On the next
track, diff the section list before and after and account for every section that
does not survive, by name, before flipping.

### 2. `handwritten.d` and schema v1 are the same debt wearing two labels

The flip failed on the first regenerate with `generated books require schema
version 2`. All thirty chapter 1–5 lessons were v1. This is not incidental: a
chapter is hand-written *because* its lessons were never migrated, and it stays
unmeasured *because* it is hand-written. `chapters.ts` already documents the
loop — a v1 chapter's payoff has no atoms to close against, and "the fix is the
schema-v2 migration, not a looser gate."

So the real cost of retiring a hand-written chapter is the v2 migration of its
lessons, not the owner file. Budget for that. The remaining tracks in
`handwritten.d` — arabic, french, german, italian, kannada, malayalam, marathi,
persian and the rest, 64 owners after this change — should be checked for schema
version *first*, because that is what decides whether the job is an afternoon or
a week.

### 3. Retiring a chapter makes latent lesson bugs load-bearing

Two bugs had sat in these lesson sources harmlessly for as long as the chapters
were hand-written, because nothing ever rendered them:

- an unescaped `*` in a PIE citation (`*nem-`) opened a markdown italic run that
  swallowed three sentences of the *namaste* etymology;
- `*h₃nómn̥` used a combining ring below (U+0325), which has no precomposed
  form, so NFC leaves it decomposed and the vendored book font cannot render
  it. `glyph-coverage.test.ts` went red the moment chapter 2 became generated.

Neither is visible in the lesson markdown to a reader skimming it, and neither
would be caught by parity or by `validate`. **Run `check:books`, the glyph
gate, and an actual XeLaTeX compile as part of the flip, not after it** — the
compile is the only thing that proves the LaTeX is valid, and on macOS
`check-book-compile.sh` needs bash 4, so materialize the compile inputs with
`book-cli.js --materialize-compile-inputs=DIR` and drive `latexmk` by hand.

### What the honest numbers did

Worth recording because two of them went the "wrong" way and both are correct:

- script-closure violations 31 → 21 for the track, and **10 → 0 across chapters
  1–5**; corpus-wide 495 → 485.
- atom-measurement-blind lessons 30 → **0**. Sanskrit is now fully measurable.
- `atomChapterSpikes` 2 → 7. This is **newly visible debt, not new debt**: those
  five chapters always taught 13–22 atoms against a ceiling of 12, and nobody
  could see it while they were v1. Chapters 6 and 7 already sat at 16 and 15, so
  the density is a property of the whole front of this book. Splitting the five
  chapters is the fix, and it is deliberately not in this change because it
  ripples into the language-ladder chapter counts.
- `firstWritingPracticeAt` 0 → 2. Gloss-first and the writing-ramp gate disagree
  here by design: the reader meets *namaste* and *namaskāraḥ* by ear, and the
  letter न arrives third. The directive is explicit that gloss-first wins, so
  this is the intended shape and should not be "fixed" by moving a writing
  microstep into lesson one.

`script-closure.test.ts` pinned `violations > 500` as a FLOOR under corpus debt.
Fixing debt broke it. That file's own comment already calls this the wrong
orientation — "debt assertions belong the other way up... it may fall, never
grow" — so the assertion is now a ceiling at 485 plus a separate floor that
keeps the test's stated point (closure finds far more than the pace budget).
**Any pin that fails because the corpus got better is pointing the wrong way.**
