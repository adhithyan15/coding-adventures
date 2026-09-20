## Chapters 79 and 80 — every character that was left

`KA-A1-L-09` closes. Kannada A1 coverage **197/258 → 198/258**; the script
column **11/16 → 12/16**.

**Characters the corpus prints without teaching: 13 → 0.** That number was 27
when the point was first measured, 19 after chapters 67–73, and 13 after
chapter 78.

| metric | before → after |
|---|---|
| exam-point coverage | 197/258 → **198/258 (77%)** |
| characters used but untaught | **13 → 0** |
| atoms taught | 440 → 453 |
| measurable lessons | 333 → 348 |
| writing-practice lessons | 89 → 104 |
| `forwardReferences` | unchanged |
| `atomChapterSpikes` | unchanged (3) |
| `durationViolations` | unchanged (0) |
| `atomsNeverRevisited` | unchanged |
| `payoffSurprises` | unchanged (0) |
| reinforcement-window misses | 707 → 745 |

### Thirteen characters, three ideas

Thirteen shapes is a lot to hand a reader. They are three ideas, and the
chapters are built on the ideas rather than the shapes.

**A vowel can be long, or it can move** — the signs ◌ೀ, ◌ೈ and ◌ೌ. ◌ೀ is the
most-used character in the whole book that nothing had taught: it is in ನೀವು,
in ನೀಲಿ, and twice inside the polite ending of *how are you?*.

**The tongue can curl back**, and Kannada writes that position where English
marks nothing — ಣ against ನ, ಷ against ಸ, with ಶ as the third sibilant. ಸಂತೋಷ
opens on one s and closes on another, and until now a reader could read one end
of that word and not the other.

**A consonant can have the breath let out after it**, and that accounts for
**seven of the thirteen** — ಖ ಘ ಠ ಢ ಧ ಫ ಭ, each built on a plain letter the
reader already had. ಧ opens ಧನ್ಯವಾದ, the second word this book teaches.

### Why two chapters and not one

The first draft put all thirteen in chapter 79. `core/chapter-policy.json` sets
**twelve new atoms per chapter**, and the snapshot diff showed
`atomChapterSpikes` going 3 → 4.

Splitting it fixed the metric and improved the chapter: chapter 79 carries the
six that are about vowels and the tongue, chapter 80 carries the seven that are
one move done seven times. **One idea per chapter**, which is what the budget is
there to enforce.

### Recognition, not writing, and the reason is sources

Chapter 78's six independent vowels were taught as **writing**, with a cited
Wikimedia Commons stroke-order animation printed under each.

**None of these thirteen has a sourced ductus anywhere in this project.** So all
thirteen are recognition lessons, and every one keeps the standing block quote
that refuses to say where the pen starts without a source. Nothing here invents
a stroke path to make a count move.

### The last glyph, and where it came from

After all thirteen landed, the count stood at **one**, not zero — and the one
was ಔ, the independent vowel *au*, appearing exactly once in the corpus.

**That one occurrence was in the new lesson itself.** `KA-S163-vowel-sign-au`
had a line explaining that this vowel's letter form is the single independent
vowel with no sourced stroke order, and it printed the letter to say so. A
lesson that prints an untaught glyph in order to explain why it is untaught is
still a lesson printing an untaught glyph.

The prose now names the letter instead of showing it, and the pin in
`tests/corpus/kannada.test.ts` is an **exact zero** rather than a ceiling — a
single new violation is a lesson asking the reader to decode something nobody
taught, and there is no longer a backlog for it to hide inside.

### What KA-A1-L-09's probe is

The **twenty-seven characters the point was opened for**: the eight chapters
67–73 taught, the six chapter 78 taught as writing, the thirteen these two
chapters taught as recognition. The point reopens if any one of them loses its
lesson.

Its label was a status rather than a demand — *"THE SCRIPT IS NOT CLOSED: N
characters are used but never taught"*, with N going 27, 19, 13 as tranches
landed. It now states what the point asks for.

