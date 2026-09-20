## Punjabi pre-A1 script recognition runway (Chapters 2-13)

- Took **script-closure violations from 40 to 0**, **never-taught glyphs from 8
  to 0**, and **headwords lacking romanization from 8 to 0**. Closure is
  measured in reading order, so this needed an *insertion* into the early
  chapters rather than an addition at the end; PR #13826 removing the
  per-chapter lesson-count pins from `language-ladder` is what made it possible.
- Inserted an **18-lesson recognition runway into chapters 2-13** — one or two
  lessons per chapter, at most three Gurmukhi pieces each, each placed
  immediately before the first lesson that asks the reader to decode them. Every
  lesson teaches the shape by eye and the sound by ear, names the piece
  (*mammā*, *lāvā̃*, *aḍḍak*), and builds one word the reader can already say.
  No writing is asked for: formation stays where it was, in chapters 14-36.
- Moved **Sanskrit and PIE etymons out of Gurmukhi and into IAST**. Before any
  letter lesson existed, ਨਾਮਨ੍, ਅਸ੍ਤਿ, ਮਿਲ੍, ਰਹ੍ and ਕ੍ਰੁ were the single
  largest source of untaught glyphs in chapters 2-5, and they dragged the
  subjoining sign ੍ — which no lesson in the track ever taught — into five of
  them. Sanskrit is not written in Gurmukhi; this is a scholarship correction
  that happens to retire a large share of the debt.
- Applied **gloss-first to the remaining early bodies**: where a chapter-2-to-13
  lesson printed a Punjabi word whose letters the runway had not yet reached, it
  now prints the romanization the reader can actually use. The Gurmukhi headword
  stays, with its romanization, as exposure.
- Added the **eight missing romanizations** to the schema-v1 ch4 farewell and
  ch5 verb headwords, which converts those headwords from load-bearing to
  exposure and is a real gain for the reader, not a way of hiding from the
  measurement.
- **Paid for it in drivability, and the number is the honest one.** The
  ear-drivable share falls from **43% to 40%** and lessons reachable in
  chapter-prefix order from **74 to 46**. A first draft typed the runway
  `reading` and appeared to *raise* both — but HL08's manifest requires
  `delivery: script` to coincide exactly with `type: writing`, and it is right:
  a lesson that teaches a letter teaches the hand and the eye, and HL08 states
  plainly that a `writing`-typed lesson has a pen core with nothing separable to
  set aside. **A track cannot close its script-closure debt without spending
  drivability**, because the only lessons closure credits are the ones modality
  scores as pen. That trade is now a measured fact rather than an assumption.
- Recovered most of the prefix that would otherwise have been lost by pushing
  **each runway lesson to the latest slot its own glyphs allow**: the letters
  have to arrive before the first lesson that decodes them and not one session
  earlier, so chapters 2, 4, 5, 10, 11 and 12 keep an ear-drivable opening.
  Placed naively at the head of each chapter the figure was 25; placed late it
  is 46.
- Gave the recognition atoms a **review layer rather than leaving 40 new atoms
  unrevisited**: each runway lesson rehearses the three lessons before it (its
  R1 neighbourhood, which a wedged-in lesson would otherwise push out of
  window), each early content lesson declares the letters its page shows, and
  each later FORMATION lesson declares the recognition atom for the same glyph —
  turning what was an unrelated first meeting into a measured R3/R4 review.
  **R1 misses fall 43 -> 34 and R2 87 -> 78**, both below their pre-runway
  values, and corpus-wide reinforcement is flat-to-better despite 41 new atoms
  entering it. R3 rises 130 -> 131 and R4 53 -> 82; the reason is stated in the
  pin and in `BACKLOG.d`: 16 of the 40 new atoms have no lesson 80-250 sessions
  later that puts their glyph back on a page, mostly because ten of the letters
  have no handwriting lesson anywhere in chapters 14-36.
- Kept the chapter ledger honest as the atoms arrived: chapters 4, 5 and 7 now
  assess the recognition atoms their runway sessions introduce, so Punjabi stays
  one of the tracks with **zero chapter-capability debt** rather than falling
  below the 0.5 representativeness floor. Chapters 4 and 5 remain legacy
  schema-v1 for their oral content, and their notes now say exactly which three
  atoms they do carry and why.
- Realigned `curriculum.d/path`: `PA-PATH-001S` held chapter-14 lessons at file
  position 3, so curriculum order disagreed with lesson sequence. Renumbered to
  `0265` so the two agree.
- Recorded in `BACKLOG.d` (HL-C272) what was measured and not done: the 18
  remaining R4 misses and the ten glyphs with no formation lesson, six chapters
  now over the 12-atom chapter budget, three payoffs below the representativeness
  floor, the two lexical forward references (ਮੈਨੂੰ, ਨਾ) that need a lesson MOVE
  rather than an addition, and the proof that the **pre-A1 verb floor is
  unreachable by authoring in any track**: all 40 core `VERB-*` concepts are
  owned by spine nodes at A1 or A2, so tagging a pre-A1 lesson with one
  relocates the lesson instead of filling the floor.

