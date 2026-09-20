## 2026-09-01 — The second Devanagari runway, and closure closed

- **Script-closure violations 44 -> 0 and never-taught glyphs 7 -> 0.** Corpus-wide
  closure fell 576 -> 532; every one of those forty-four was Marathi's. Of the
  four Devanagari tracks, Marathi and Marwadi now measure clean on both counts;
  Hindi still carries 68 violations and 12 never-taught glyphs, Sanskrit 46
  and 9.
- **The fix was resequencing, not addition, and that is the whole finding.**
  Closure is measured in READING ORDER, so a glyph taught late cannot retire a
  violation that happens early. Seventeen of the twenty-three offending glyphs
  were *already in the corpus* — but the earliest lesson the module could credit
  with teaching them sat at reading position 112, inside the A1 form chapters.
  Marathi did not have a coverage gap; it had an ordering gap, and no amount of
  new material at the end could have touched it.
- **Four new chapters, inserted where the debt starts.** Chapters 5-8 carry
  **twenty-four signs, one per lesson**: the marks **ि ु ू ृ ँ अ**; the velar and
  palatal rows **ख ग घ च छ ज झ**; the retroflex row plus **प** — **ट ठ ड ण प**;
  and **ल श ष उ ऊ ए**. Every later chapter moved +4 (old 5-25 are now 9-29);
  every lesson ID stayed put, so prerequisites, reviews, modality shards and
  assessment references all survived untouched.
- **Order chosen by what each sign unblocks, not by alphabet order.** The marks
  come first because they blocked the most lessons (**ु** alone appeared in 18
  violating lessons, **ि** and **ण** in 15 each). The two consonant rows are
  taught as a *pattern* — plain, breathy, voiced, voiced-and-breathy — so a
  reader who forgets a letter can rebuild it rather than re-memorise it. **ल**
  arrives after **ळ** on purpose: the rarer retroflex was needed first, by
  *kāḷjī* and *ḍoḷā*.
- **Every citation was verified to exist before the lesson was written.**
  HL-C217 refused to author **ग, घ, ख** rather than invent a Wikimedia Commons
  URL to match the house pattern. This tranche queried the Commons API for all
  twenty-four signs first: fifteen consonants have `File:Deva-<glyph>-order.gif`
  by Opiaterein, the independent vowels **अ उ ऊ ए** have
  `File:Devanagari <glyph> stroke order.svg` by Saurmandal, and the five marks
  have neither — so they cite the Unicode Devanagari chart with their
  codepoints, exactly as the track's existing mātrā lessons already do. HL-C217
  is closed by this tranche.
- **The twenty-four new atoms are reviewed at every window.** Each chapter
  closes with an ear-only retrieval payoff that adds no sign, and **nine
  ear-only review lessons** in chapters 9, 13 and 22 carry the set through R2,
  R3 and R4. All twenty-four now hit R1, R2, R3 *and* R4 — zero window misses.
  Marathi's overall missed windows fell R2 71 -> 62, R3 95 -> 80, R4 67 -> 54,
  and its average lessons-per-SCRIPT-atom rose 5.3 -> 6.5, against 5.0 for LEX
  atoms. That answers the corpus-wide finding that script atoms are reviewed
  roughly a fifth as often as vocabulary atoms.
- **The modality cost is recorded, not engineered away.** Twenty-four sign
  lessons are `type: writing`, which `modality.ts` derives as `pen` before it
  looks at any block, so no detachable segment can rescue them. The drivable
  share fell **61% -> 56%** (voice 52 -> 63, sight 52 -> 54, pen 64 -> 88).
  Retyping those lessons to get a voice core out of the derivation would have
  bought the number and lied about the lesson: detach the script block from a
  lesson whose subject *is* the letter and nothing is left. The nine ear-only
  retrieval lessons are where the ear is paid back honestly.
- **One schema-v2 lesson now lives in a legacy hand-written chapter.**
  `MR-R09-runway-tail-r2` had to sit at reading position 71 to land in the R2
  window for chapter 8's last six signs, and that is inside chapter 9, whose
  book text is hand-written. It is declared in `embeddedLessonIds` and carried
  in `ch09-introductions.tex` with a `% canonical-insertion:` marker and a
  `\label{lesson:...}` — the mechanism HL05 already provides, used by Punjabi
  chapter 4 for the same reason.
- **What did not move, deliberately.** Pre-A1 vocabulary is still **48/300**, and
  is now unambiguously the track's only remaining pre-A1 blocker. A vocabulary
  tranche would have moved it to about 60; this tranche bought closure instead,
  because forty-four lessons showing untaught letters is a broken book while
  forty-eight headwords is merely a short one. The eight headwords without
  romanization are still eight and should stay: they are chapter-25 writing
  tasks where withholding the romanization is the exercise.
- Added thirteen sound tags, four chapter capability entries, five curriculum
  path segments and seven extension nodes; regenerated the book, narration,
  modality, gentle-ramp and assessment artifacts; and updated the pinned
  Marathi corpus tests to the new chapter map. Lessons 168 -> 205, chapters
  25 -> 29.

