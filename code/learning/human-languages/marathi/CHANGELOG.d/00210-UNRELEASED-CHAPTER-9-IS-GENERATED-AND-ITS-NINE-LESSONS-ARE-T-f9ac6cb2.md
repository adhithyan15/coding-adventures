## Unreleased — chapter 9 is generated, and its nine lessons are typed (HL-C287)

`book/chapters/ch09-introductions.tex` is now generated from its ten lessons.
It was the first of Marathi's four hand-written chapters; three remain (10, 11,
12), and the whole hand-written island is the same shape: 26 schema-v1 lessons
sitting between generated chapters 1–8 and 13–29.

- **The migration was the work.** All nine content lessons were schema v1,
  which declares no knowledge atoms, and `book.ts` refuses to generate from a
  v1 lesson. They now carry v2 frontmatter, an `hl-knowledge` directive on
  every body block, and seventeen atoms between them. Their headings already
  classified — `The letters in this word`, `The word, taken apart`,
  `Grammar Lens:`, `Why it's said this way` — so only four landed on `unknown`
  and each was re-pointed by PREFIXING, never rewriting:
  `The phrase, assembled` → `You'll want to know — the phrase, assembled` (×3),
  `The whole exchange` → `The exchange`, and
  `Atoms banked this chapter` → `What you've built — atoms banked this chapter`.
  A per-character non-ASCII census across all nine shows **zero** Devanagari or
  IAST characters changed; the only deltas are the em dashes those renames add.
- **Two atom ids were minted alongside existing ones rather than re-pointed.**
  `MR-C07-asne` in chapter 14 already owns `MR-LEX-ASNE-AAHE` and
  `MR-GRAMMAR-AAHE-LAST` — for **आहे**, which chapter 9 teaches *first*.
  Duplicate introduction is a hard error, and re-pointing a generated chapter
  has the larger blast radius, so chapter 9 takes `MR-LEX-AAHE-COPULA` and
  `MR-GRAMMAR-VERB-FINAL`. That the corpus teaches the copula twice is real and
  pre-existing; this change makes it visible rather than causing it.
- **`MR-C02-practice` was in no path segment**, which schema v2 forbids for a
  lesson declaring a spine node. It joins `MR-PATH-005` between `MR-C02-anand`
  and `MR-R09-runway-tail-r2` — its sequence position, so reading order is
  unchanged — under a new `MR-EXT-038-INTRODUCTIONS-CONSOLIDATION`, mirroring
  how `MR-C01-practice` is already placed.
- **`chapters.d/0009.json` said the chapter "has no typed knowledge atoms yet
  ... until migration".** That is now false. The payoff names the seventeen
  atoms `MR-C02-practice` actually assesses; at five it sat at 0.28, below the
  0.5 representativeness floor, and it also understated what the recap does.
- **Three things the hand-written chapter taught and the lessons did not say
  are carried back**, found by a content-word census of the two `.tex` files
  rather than by the block gap: the section hook *"a word older than Europe"*,
  which lived in the `.tex` heading and not in the lesson's own title; the
  promise in the runway retrieval that the same six signs come back twice more;
  and — the reverse — the `.tex`'s pointer to "Chapter 4's *punhā bheṭū*",
  which is stale, the lesson's "Chapter 11" being right.
- **`ळ` is back in the runway retrieval, and the reason it was dropped was
  wrong.** The `.tex` printed the curled *ḷa* as the answer's confirmation; the
  lesson had only the words for it. It was left out on the belief that no
  lesson taught the shape — but `MR-W03-lla` owns `MR-SCRIPT-LLA-01` in
  **chapter 3, sequence 31**, 270 positions earlier. The glyph is restored and
  the atom declared, which closes its R3 reinforcement window: script-closure
  violations do not move and `reinforcementWindowMisses` falls by one.
- **Two defects only the generated book could show.**
  `MR-C02-naav` cited PIE **\*h₃nómn̥** with U+0325, COMBINING RING BELOW.
  Latin Modern Roman does not have that glyph — verified by reading the cmap of
  `lmroman10-regular.otf`, not assumed — so it would have reached the reader as
  a missing character. The reconstruction stays; the syllabic *n* is now said
  in words instead of marked with a glyph the book cannot set. And
  `MR-C02-aahe` told the reader it would meet *āhes*/*āhāt* "in Chapter 9"
  while sitting in chapter 9; per HL-C102's own rule the fix is to name the
  thing, so it now says "in the next chapter".

Two pre-flight checks, run before planning and worth repeating for 10-12:

- **The honest denominator is the lessons, not the `.tex`.** German chapter 4
  owned three writing lessons staged separately and appearing nowhere in its
  `.tex`, so the section count understated the chapter by a third.
  `grep -l '^chapter: N$' lessons/*.md` gives 10 / 6 / 6 / 5 for Marathi
  chapters 9-12 against `.tex` section counts of 10 / 6 / 6 / 5 — an exact
  match, so Marathi stages no hidden writing lessons in this island. Confirmed
  rather than assumed.
- **A v1 lesson can carry a glyph that is free hand-written and fatal
  generated.** A character is safe only if it is in the track's own script
  block, in `main-font-charset.json`, mapped by `\newunicodechar` in the
  preamble, **or rewritten by `book.ts`'s escape map** — that last arm is the
  one a naive scan misses, and it covers every arrow, subscript and modifier
  letter chapters 10-12 use. With all four arms applied, chapters 10-12 hold no
  unsettable character; with the escape-map arm removed the same scan still
  flags U+0325, the one that was real.

Counters, each re-measured against the tree:

- Marathi's lesson-content budget moves **179 → 188**. Not one is a new lesson;
  all nine were already written and already in the book. Idioms, senses and
  culture claims are unchanged at 5 / 4 / 7.
- `atomsTaught` **168 → 186**; `atomMeasurementBlindLessons` **28 → 19**.
- `atomChapterSpikes` **0 → 1** — a legitimate RISE. Chapter 9 introduces 17
  atoms against a `maxNewAtomsPerChapter` of 12. No lesson exceeds the per-
  lesson limit of 3; the chapter is genuinely where the whole introduction
  exchange lands. Declaring fewer would mean under-declaring what the lessons
  assess, so the report-only gate records it.
- `reinforcementWindowMisses` **218 → 267**, also a legitimate rise: newly
  declared atoms are newly measured ones, and the debt predates the declaration.
  Re-measured against the merged tree; an earlier pre-merge reading of 271 was
  three too high, which is what "never derive a counter" is there to prevent.
- `atomsNeverRevisited` is back to its pre-existing **1**. It went to 4 first,
  and each of the three was a real gap the recap already closed in prose but
  not in its directives — the *m*→*v* softening, the respectful possessive, and
  the courtesy-by-plural rule are all re-banked by lessons that had not declared
  them.
- `forwardReferences` is unchanged at **4**. It can move in either direction
  when a chapter starts teaching words earlier chapters previewed, so it was
  measured before and after rather than assumed steady.
- Marathi stays in the "chapter debt is already zero" list; it dropped out
  mid-change on payoff representativeness and is back on merit, not by editing
  the list.

Verified: human-language-data 124 test files / 1730 passing; all eleven
`check:*` gates; language-ladder 39 files / 442 passing; the whole Marathi book
compiles under XeLaTeX with zero overfull boxes, and chapter 9's pages were read
as rendered PDF — Devanagari conjuncts, the standalone mātrā, the three-column
exchange table and the atom-bank box all set correctly.

