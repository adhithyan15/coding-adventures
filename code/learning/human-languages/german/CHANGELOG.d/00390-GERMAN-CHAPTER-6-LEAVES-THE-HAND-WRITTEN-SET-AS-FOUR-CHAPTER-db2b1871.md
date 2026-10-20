## German chapter 6 leaves the hand-written set, as four chapters

Chapter 6 — the numbers one to ten — is now generated from its lessons, as
**chapters 6, 7, 8 and 9**. German's hand-written chapters: **8 -> 7**. Old
chapters 7-36 renumber to 10-39.

### Two lessons were holding ten numbers

| Method | Said |
|---|---|
| `handwritten_parity.py german` | a gap of **1 block** (5 in the `.tex`, 6 in the lessons) |
| `grep -l '^chapter: 6$' lessons/*.md` | **2 lessons**, and the `.tex` renders **2 sections** |
| the German the `.tex` teaches vs. the lessons that own it | **ten numbers owned by two lessons, five each**, plus *ein*/*eine*, the sound law, the *-cht-*/*-ght-* correspondence and the Latin-months observation owned by nobody — **fourteen items, two owners** |
| **reading the tables** | two **four-column** tables, five rows each, against a `maxLinearisableTableColumns` of **3** |

The first two methods were the misleading ones again, and for a new reason:
this chapter's `.tex` is honest about its own size. Five blocks, two sections,
two lessons — everything agrees, and everything is wrong, because *one lesson
teaching five numbers* is five items in one sitting no matter how tidy the
prose around it looks. The rule is one new item per lesson, and length is never
a cost.

**2 lessons became 16**: one per number, the sound law that makes the
differences predictable rather than random, the article *ein*/*eine*, a run for
each five, the seam between homegrown numbers and imported month names, and a
closing exchange. All sixteen are `voice` and drivable.

### The four-column tables are gone, and the narrator notices

Both of the old tables were `German | English twin | shared ancestor | Latin
cousin` — four columns, which `chapter-policy.json` calls unspeakable and the
narration lineariser refuses outright. `narration.test.ts` counts corpus-wide
refusals and it moves **51 -> 49**, the same shape and the same cause as the
French chapter 6 entry sitting beside it in that pin.

What replaced them is not a narrower version of the same grid. Each number now
carries its own English twin in its own lesson, and the two recap tables are
three-column runs (*German | Said | English twin*) that a narrator can read
aloud.

### The reconstructions went; the sound law stayed

The `shared ancestor` column held ten Proto-Indo-European reconstructions.
Every one is gone, and none of the teaching went with them. A beginner counting
to ten cannot use a reconstruction; what they can use is the *rule* the
reconstructions were evidence for, which now has a lesson of its own — German
and English are siblings, Latin is a cousin, and a sound law is a change that
caught every word at once, which is why *drei*/*three*/*tres* differ
predictably.

That lesson also connects to something the reader already owns: *gut* from
*good* and *machen* from *make* are the **later**, German-only shift, and
*zehn* is the one number where both shifts are visible in sequence — an old
**d** to **t**, which is where English stopped, and then **t** to **ts**, which
is where German went on.

### Two examples were swapped for words the reader has

*ein Kaffee* and *eine Katze* illustrated the article with two nouns the book
does not teach until chapters 28 and 22. They are now *ein Tag* and *eine
Nacht*, from *guten Tag* and *gute Nacht*, which the reader has owned since the
greetings. Same point, no forward reference, and the *der/die* split it depends
on is already theirs.

### What moved

- `handwritten.d/german-0006.json` is now a `targets.d/` entry.
- `handwritten_parity.py german`: **40 -> 39** blocks at risk.
- **No path shard was renumbered.** All sixteen lessons sit on the existing
  `GE-PATH-015`, so `curriculum.d/path/` is untouched apart from that node's
  own lesson list.
- One sequence moved outside the chapter: `GE-C07-wochentage-1` from 225 to
  227, because sixteen lessons need 211-226 and chapter 7 was sitting at 225.
  Its lesson id did not move.
- Both old lesson ids were **kept**, as the two recap lessons.
  `GE-C06-zahlen-1-5` and `GE-C06-zahlen-6-10` are referenced as prerequisites
  or reviews by six lessons in chapters 7, 8, 9, 12 and 31; retiring the ids
  would have broken every one of them, and a run of five numbers is a real
  thing to practise rather than a placeholder.

### Counters, re-measured against the merged tree rather than composed

| Measure | Before | After |
|---|---|---|
| German hand-written chapters | 8 | **7** |
| atoms taught | 335 | **367** |
| lessons with measured budgets | 181 | **197** |
| atom-measurement-blind lessons | 16 | **14** |
| chapters over the 12-atom budget | 6 | **5** |
| culture claims | 18 | **19** |
| forward references | 36 | **33** |
| atoms never revisited | 78 | **83** |
| reinforcement window misses | 687 | **800** |
| `handwritten_parity.py` german | 40 | **39** blocks at risk |
| corpus narration refusals | 50 | **48** |
| book pages | 346 | **367** |

RE-MEASURED ON RETRIEVAL. This chapter was authored before German chapters 14,
15, 16, 17 and 19 landed, so every "before" above was recomputed against the
merged tree rather than carried over from the original commit. `paradigmTables`
(95), `lessonsWithFindings` (121), `ruleStatements` (30) and `fullParadigmGrids`
(21) are all unchanged.

### Thirty-two atoms was a cram, and the band that justified it was the wrong band

This chapter first shipped as ONE chapter of **32 atoms**, argued from the five
chapters beside it: German's opening runs 31, 23, 36, 30, 30. That argument was
measured against a **sub-population**, not the track.

German's actual band, across all 29 atom-bearing chapters at the time:

| | German | French |
|---|---|---|
| median | **9** | 9 |
| mean | 12.7 | 8.5 |
| max | 36 | 12 |
| over the ceiling | 6 of 29 | 0 of 37 |

German's median is the same as French's. The six over-budget chapters are the
opening band, and **the opening band is debt, not a design** — 23 of 29 chapters
already sat at or under the ceiling. Against that, 32 is nearly three times
`maxNewAtomsPerChapter`, and the rule is not ambiguous: if a chapter cannot fit
its material, split it; never cram, never raise a ceiling.

So it is four chapters, cut at the `.tex`'s own 1–5 / 6–10 seam and then again
inside each half, because halves of 18 and 14 were both still over:

| chapter | atoms | lessons |
|---|---|---|
| 6 One, Two, Three | **9** | 4 |
| 7 Four, Five, and the Article Hiding in *Eins* | **9** | 4 |
| 8 Six, Seven, Eight | **8** | 3 |
| 9 Nine, Ten, and the Months That Are Numbers | **6** | 5 |

`atomChapterSpikes` moves **6 → 5** rather than 5 → 6: chapter 6 leaves the
over-budget list instead of joining it, and the only German chapters still above
the ceiling are the five nobody has split yet.

German stays pinned at **zero** cross-chapter prose references. Three drafts
said "since chapter one"; all three now name the thing instead — "since your
first *danke*", "the two nouns *guten Tag* and *gute Nacht* are built from".

### Deliberately not carried over

The ten PIE reconstructions, and Latin *unus* and *duo*, which added a fourth
column without adding a claim the reader can use. Everything else the
hand-written chapter taught is present: all ten numbers with their English
twins, both sound-law showpieces, the *-cht-*/*-ght-* fossil, the article, and
the Roman calendar's off-by-two months. The German month spellings themselves
stay in chapter 9, where they are taught, and the lesson says so rather than
previewing them.


