## HL-C272 — closure is bought by moving letters earlier, and what that cost Marathi

Marathi's script closure went **44 violations → 0** and its never-taught glyphs
**7 → 0** in one tranche, leaving Marwadi as the only other Devanagari track
measuring clean on both. The interesting part is not the number; it is that the
obvious way to move it does not work, and the way that does work is expensive in
a specific, predictable place.

**The measurement, restated.** `script-closure.ts` walks a track in READING
ORDER and asks, for each glyph a lesson puts in load-bearing text, whether an
earlier lesson taught it. "Earlier" is `(chapter, sequence)`. A glyph taught in
chapter 20 does nothing for a lesson in chapter 6 — Bengali proved this by
adding thirty-five lessons and moving closure 65 → 65.

**What Marathi's forty-four violations actually were.** Twenty-three distinct
glyphs caused them. **Seventeen of the twenty-three were already in the corpus**
— but the earliest lesson the module could credit with teaching them sat at
reading position 112, in the A1 form-label and named-reader chapters, where
`delivery: script` lessons happen to contain those letters incidentally. Only
six were genuinely absent. So the track did not have a *coverage* gap. It had an
*ordering* gap, and no amount of new material at the end could touch it.

**The fix is structural, not additive.** Four new chapters (5–8) carrying
twenty-four sign lessons, inserted between the courtesy chapter and the
introductions chapter, with every later chapter renumbered +4. That is the
shape: a second runway placed where the debt starts, not appended where the
author happens to be working. Lesson IDs did not change, so prerequisites,
reviews, modality shards and assessment references all survived; what changed
was `chapter:` in 135 files, the chapters/targets/handwritten shards, and the
`.tex` filenames.

**Things that broke on the way, worth knowing before the next renumber:**

- `sequence` must stay monotone with `chapter` across the whole track
  (`chapters.test.ts`). Inserting chapters means re-basing every later sequence;
  Marathi shifted everything ≥ 60 by +200 and gave the runway 60–96.
- Hand-written `.tex` chapters carry `\hlchaptermodality{N}` in their body. The
  file rename does not fix it and `chapter-modality-book.test.ts` catches it.
- `generated-book-hashes/<lang>.d/` shards are not cleaned by the generator.
  After a renumber the old ordinals sit there as stale files and
  `check:shards` fails with a set difference, not a content error.
- Hand-editing a `chapters.d` or `targets.d` shard with `json.dumps` defaults
  escapes non-ASCII and the canonical-shard test fails on `च` vs `च`.
- A schema-v2 lesson landing in a legacy hand-written chapter needs an
  `embeddedLessonIds` entry plus a `% canonical-insertion:` marker and a
  `\label{lesson:<id>}` in the `.tex`. That mechanism already exists (Punjabi
  ch4 uses it); it is the right answer, not a workaround.

**The cost, recorded rather than engineered away.** Twenty-four sign lessons are
`type: writing`, which the modality module derives as `pen` unconditionally and
which no detachable segment can rescue: `coreDerived` returns `pen` for a
writing-type lesson before it looks at any block. Marathi's drivable share fell
**61% → 56%**. That is honest. A letter lesson whose script block is detached
teaches nothing, so labelling these `voice` — or retyping them `reading` to get
a voice core out of the derivation — would have bought the metric and lied about
the lesson. **Anyone closing another track's script debt should expect the same
five-to-six-point drop and should not go looking for a way around it.**

What *can* be done, and was: nine ear-only retrieval lessons (`type: review`, no
script block, romanization only) carry all twenty-four atoms through R2, R3 and
R4. Those are genuinely drivable, they lifted Marathi's voice count 52 → 63, and
they answer the separate corpus-wide finding that script atoms are
under-reviewed — Marathi's average lessons-per-SCRIPT-atom is now 6.5 against
5.0 for LEX atoms, where before it was 5.3.

**Citations were verified before writing, not after.** HL-C217 refused to author
**ग, घ, ख** because no stroke-order source was on hand and inventing one to fit
the house pattern would be worse than a measured gap. This tranche queried the
Commons API for all twenty-four signs first: fifteen consonants have
`File:Deva-<glyph>-order.gif` by Opiaterein, four independent vowels have
`File:Devanagari <glyph> stroke order.svg` by Saurmandal, and the five marks
have none — so the marks cite the Unicode Devanagari chart with their
codepoints, which is what the track's existing mātrā lessons already do. **Run
the API query before planning the lessons**: which signs have animations decides
which lessons can be shape-first and which have to be placement-first.

**What this leaves.**

- Marathi's pre-A1 vocabulary is still **48/300** and is now unambiguously the
  track's only remaining pre-A1 blocker. This tranche deliberately bought
  closure instead, because 44 lessons showing untaught letters is a broken book
  and 48 headwords is a short one.
- The eight headwords without romanization are still eight, and should stay
  eight: they are chapter-25 writing tasks where withholding the romanization is
  the exercise.
- The corpus still has **532** closure violations across the other non-Latin
  tracks. The Marathi shape — measure which glyphs block which lessons, check
  the citations exist, then insert chapters where the debt starts — transfers
  directly. Ranked by violations today: Hindi 68, Bengali 65, Russian 59,
  Arabic 57, Persian 47, Sanskrit 46, Urdu 46, Punjabi 40, Kannada 30, Tamil 29,
  Telugu 22, Malayalam 19, Chinese 4. Hindi and Sanskrit are the obvious next
  candidates because they share Devanagari with the runway just written, so the
  citation survey is already done for twenty-four of their signs.
- `minLessonsBetweenScriptSegments` is 2 in `chapter-policy.json` and this
  runway ignores it, as chapter 1 already did. Either the policy is wrong for a
  track that has to close a script debt in one pass, or the runway should be
  interleaved with content lessons. It is report-only today and nothing measures
  it per track; that is the honest gap.
