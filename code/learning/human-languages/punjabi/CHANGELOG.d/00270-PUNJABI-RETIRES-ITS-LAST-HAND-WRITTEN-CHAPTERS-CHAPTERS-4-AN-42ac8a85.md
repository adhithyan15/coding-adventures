## Punjabi retires its last hand-written chapters (Chapters 4 and 5)

- **No hand-written book chapters remain in the Punjabi track.** Chapter 4
  (farewells) and Chapter 5 (first verbs) moved from
  `core/book-generation.d/handwritten.d/` to `targets.d/` and are now generated
  from their lessons. `handwritten_parity.py --check punjabi` answers "already
  retired, nothing handwritten remains".
- **The flip required a schema-v2 migration, and that was the real work.** A
  generated chapter cannot be built from a schema-v1 lesson, so all ten legacy
  lessons behind those chapters now declare `spine_node`, a computed-safe
  `duration`, typed `requires`/`introduces`/`practises` knowledge, block-boundary
  knowledge directives and scored activities. Punjabi is now a **version-2
  track** rather than a mixed-schema one, and its atom-measurement-blind lessons
  fall from **18 to 8**.
- **Split the two Chapter 5 lessons that packed several headwords into one
  session.** `PA-C05-main-punjabi-bolda-han` had been teaching the word
  *panjābī*, its Persian etymology, the gendered present habitual and the whole
  sentence at once; `PA-C05-kamm-karna` had been teaching the noun *kamm*, the
  verb *karnā*, the √kṛ root and the noun-plus-*karnā* pattern. Two new lessons,
  **PA-C05-panjabi** and **PA-C05-karna**, take the first half of each. Every
  Chapter 4 and 5 lesson now introduces **at most three atoms and exactly one new
  headword**, and every one comes in under the computed five-minute ceiling (the
  longest is 240s against a 300s limit). The track grows 224 → 226 lessons.
- **Script closure stays at 0 and romanization coverage stays complete.**
  `headwordsWithoutRomanization` is **0 before and 0 after**; reading-order
  closure violations are **0 before and 0 after**. Gloss-first is preserved by
  keeping the script sessions where the lesson sequence puts them rather than
  where the hand-written .tex put them: the reader meets *phir* and *bolṇā* by
  ear, and their pieces arrive later in the same chapter. That placement is also
  what the script lessons' own prose has always assumed — they speak of a word
  "you already say" and ask the reader to look *back* at an earlier headword.
- **Reinforcement debt rises because it became visible, not because it grew.**
  Twenty-five atoms the corpus taught in prose and counted nowhere are now typed,
  so every R1-R4 window they miss is now measured: R1 45→48, R2 103→113, R3
  158→175, R4 72→92. The assertions that name specific serviced atoms all still
  hold exactly. The residue is recorded in `BACKLOG.d` as the next tranche's work.
- **Chapter 5 carries 15 new atoms against a chapter budget of 12**, one new
  atom-chapter spike (Punjabi 11 → 12). That is the shape Chapter 3 (17) and
  Chapter 6 (13) already have; clearing it means splitting the chapter, which
  renumbers Chapters 6-36 and belongs to a separate tranche.
- Fixed two defects the flip exposed in text that had never been through the
  generator: a stray `>` left by a blank blockquote-continuation line in the
  script sessions, and `[YOU SAY (m.): ...]` cues whose parenthesis defeated the
  cue parser and printed as literal markup.
- The book compiles: 303 pages under XeLaTeX, with no overfull or underfull
  boxes and no missing characters.

