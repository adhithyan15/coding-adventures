## Unreleased — Kannada owns no hand-written chapters

Chapters 1, 2 and 4 are generated. With chapters 3 and 5 already retired,
`handwritten_parity.py --check kannada` now answers **"already retired, nothing
handwritten remains"** and exits 0. The track is closed.

Nineteen more `KA-C0[124]-*` lessons moved from schema v1 to v2, gaining 37
knowledge atoms. The three `practice` lessons sat in no curriculum path segment,
which v2 forbids for a lesson declaring a spine node; each is appended to the
segment that already holds its prerequisites (`KA-PATH-003`, `-004`, `-009`)
with a matching consolidation extension, so no segment had to be inserted and no
path shard had to be renumbered.

### The pipeline learned the block type the corpus had already authored

Two lessons — `KA-C01-dhanyavada` and `KA-C01-illa` — carried a heading
`## Across the family — the same idea, five ways` that **no block type matched**.
It classified as `unknown`, which schema v2 rejects, so those two lessons could
never have been generated. The books have drawn that comparison in their own
violet `cognates` box since the beginning; the pipeline simply had no name for
it. `parse.ts` now classifies it, exactly as it already does for
"The letters in this word", and `book.ts` renders it.

It renders as an **inline** `tcolorbox` rather than `\begin{cognates}`: only the
four Dravidian preambles define that environment, so emitting the named one
would compile here and break the moment any other track used the heading. The
colours travel in the emitted LaTeX instead, so the page is unchanged.

### Prose the parity script could not see

`handwritten_parity.py` reported 6 blocks at risk. Six were at risk — but not
the six it named. It compares COUNTS per environment, so chapter 1's two
`culture` blocks matched its lessons' two "Why it's said this way" sections and
the gate reported nothing, while in fact **one .tex block had no lesson and one
lesson block was not in the .tex**: equal counts, different sets.

Matching blocks by CONTENT instead (Jaccard over tokens, which survives the
rewrapping the prose underwent on its way into LaTeX) found the real list:

| reported by parity | actually orphaned |
|---|---|
| ch1 `sounds` ×1 | **false** — `KA-C01-namaskara`'s script block already covers it, *ottakṣara* included, and that term is also in `KA-C01-practice` |
| ch1 `cognates` ×3 | **1** — two were already authored in the lessons; only the *namaskāra* "Hello" table was missing |
| ch1 — *(not reported)* | **1 `culture`** — the "go and come back" farewell |
| ch2 `cognates` ×1 | **1** — the *hesaru* "Name" table |
| ch2 — *(not reported)* | **1 `cousinweb` clause** — the \*nān pronoun stem behind Tamil *nāṉ* / *eṉ* |
| ch4 `cognates` ×1 | **1** — the "go and come back" table |

Three cognate tables were carried across from the `.tex`, plus the *nān stem
clause into `KA-C02-nanna`. The chapter-1 `culture` block was **not** copied
anywhere: `KA-C04-hoogi-baruttene`'s own etymology block already teaches all of
it. It was added, the page was compiled and looked at, and the same idea printed
three times on one spread — so it came back out. Measured at 64% token overlap
with zero unique facts.

### Four forward references resolved, deliberately

Generating chapters 1 and 2 drops four Kannada words from them — ಹೋಗಿ,
ಬರುತ್ತೇನೆ, ಬನ್ನಿ and ನಾನು. That is the point. `KA-C01-practice` already says so
in its own words: the departure exchange *"belongs in the later chapter where
you learn those actions one at a time."* The hand-written `.tex` taught them
three chapters early anyway. All four survive in the generated book (11, 13, 1
and 25 occurrences) and each now first appears in the chapter that teaches it.
`measureContinuity` reports Kannada's forward references at **13**, unchanged.

### Two more defects only the page showed

Ten further lessons carried the duplicated `Sounds you'll need` / `The letters in
this word` pair described in the previous entry; all ten are resolved the same
way, each proven a strict subset first. Three lessons — `KA-C02-enu`,
`KA-C04-hoogu`, `KA-C04-naale-sigona` — were not NFC-normalized and now are,
which completes the seven the corpus had.

`KA-C01-practice`'s recap table was **four columns with an unlabelled second
column** holding the romanization — the exact shape `chapter-policy.json` calls
unspeakable, and the narrator refused it. The romanization now sits beside its
Kannada in the `Read` cell, making three labelled columns, so the narrator reads
the table instead of refusing it. The corpus refusal pin moves 54 → 53.

### One bug fixed outside the track

`hindi/lessons/HI-C03-thik.md` had raw LaTeX leaked into its Markdown —
`long *\=\i*` where it meant `long *ī*`. Hindi chapter 3 is already generated,
so the Hindi book has been **printing `long =\i` to readers**. Found by grepping
the corpus for the same leak after `KA-C02-niinu-niivu` turned out to have one;
those two files were the only cases in 22 tracks.

### All five payoffs were below the representativeness floor

`chapter-policy.json` sets `payoffRepresentativeness: 0.5` — a chapter's payoff
should assess at least half of what the chapter introduced. Every one of the five
newly generated chapters failed it, because each payoff had been anchored to a
hand-picked highlight from the practice lesson rather than to what that lesson
actually does:

| chapter | was | now |
|---|---|---|
| 1 | 5/14 = 0.36 | 8/14 = **0.57** |
| 2 | 3/17 = 0.18 | 11/17 = **0.65** |
| 3 | 2/15 = 0.13 | 11/15 = **0.73** |
| 4 | 2/12 = 0.17 | 9/12 = **0.75** |
| 5 | 3/9 = 0.33 | 8/9 = **0.89** |

The payoff *is* the practice lesson, so `assesses` is now everything that lesson
practises which this chapter introduced — no widening of what is taught, just an
honest reading of what the closing checkpoint already covers. The corpus-wide
count of payoffs below the floor moves **91 → 86**, measured before and after.

This was invisible while the chapters were v1: a schema-v1 lesson declares no
atoms, so the denominator was empty and the floor could not be computed at all.

Counted from the directory of the merged tree, never derived: `handwritten.d`
holds **31** entries on `origin/main` and **28** on this branch. **Kannada's
count is 0.** The corpus-wide figure has moved 38 → 31 while this work was in
flight, as Arabic, Urdu and several French and German chapters were retired
alongside it, which is exactly why it is re-counted rather than adjusted.

The corpus-wide narrator refusal pin moves to **51**, measured against the
merged tree rather than derived: main had already moved 54 → 52 for French
chapter 7 while this branch held 53.

Verified: human-language-data 124 test files / 1730 tests, all eleven `check:*`
gates, language-ladder 39 files / 442 tests, and the whole Kannada book compiled
under XeLaTeX — 433 pages, **zero missing characters**, zero overfull and zero
underfull boxes. The pages carrying every rescued block were rasterised and read.

