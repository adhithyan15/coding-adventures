## German chapter 5 leaves the hand-written set

Chapter 5 — the first verbs — is now generated from its lessons. German's
hand-written chapters: **11 -> 10**.

### The hole was a paradigm, not a word list

Three of the four sizing methods understated this chapter, and the fourth is the
one that found the work:

- `handwritten_parity.py german` scored chapter 5 at a gap of **4 blocks**
  (12 in the `.tex`, 9 in the lessons).
- `grep -l '^chapter: 5$' lessons/*.md` gave **5 lessons**, and the `.tex`
  rendered **5 sections** — no writing lessons hidden off the page this time,
  unlike chapter 4.
- Counting the German the `.tex` teaches against the lessons that own it found
  **eight items owned by nobody**: *er*, *sie* (she), *wir*, *ihr*, *in*, *wo*,
  *was* and *Deutsch*. Three of them — *wo*, *was* and *in* — were used in the
  chapter's own closing dialogue and taught nowhere at all.
- Reading the chapter's **tables** is what sized it. Three of its four tables
  are six-row person paradigms, and `maxNewGrammarCellsPerLesson` is **1**. A
  row-per-person table is a paradigm, and no grep reports that.

`GE-C05-wohnen` alone introduced the verb, a six-row present-tense grid, and
five pronouns nobody had taught — including the *-st* that *du heißt* had been
hiding behind an eszett. Against `maxNewAtomsPerLesson: 3` and
`maxNewGrammarCellsPerLesson: 1` that is one lesson doing about eight lessons'
work.

Chapter 5 is now **15 lessons**: the verb, the stem rule, one lesson per new
pronoun with the ending it takes, the audibility contrast, the three question
and place words, the three verbs, the first self-assembled sentence, and the
overt-subject rule. All fifteen are `voice` and drivable, so the chapter keeps
its hands-free start.

### Why it is one chapter and not two

Chapter 5 introduces **30 atoms**, over `maxNewAtomsPerChapter`. It was not
split, and that is a deliberate call rather than an oversight: German chapters
1-4 are already generated and merged at **31, 23, 36 and 30** atoms, because
this track's opening word lessons each carry a sound atom and an etymon atom
alongside the word. Chapter 5 at 30 sits exactly with its four neighbours, so a
split here would buy consistency with the ceiling at the cost of consistency
with the four chapters either side of it — and would renumber every later German
chapter while chapters 10-15 are being retired in parallel. `atomChapterSpikes`
moves 4 -> 5 and records the debt where the whole opening range already sits.
Chapter 16 was split, and was split because a `sein` paradigm genuinely does not
fit; this is the other case.

### What moved

- `handwritten.d/german-0005.json` is now a `targets.d/` entry.
- `handwritten_parity.py german`: **55 -> 51** blocks at risk.
- `SPINE-SAY-WHAT-I-DO` needed a **segment split**: *machen* carries the
  canonical concept `VERB-DO-MAKE` and therefore already sat on
  `SPINE-NAME-EVERYDAY-ACTIONS` in its own path node, while six chapter-5
  lessons come after it. The path is now `GE-PATH-014` (through *wo*) ->
  `GE-PATH-A1-MACHEN` -> `GE-PATH-014-B` (from *was*), with the spine's segment
  ledger updated to match. Inserting that path shard renumbered the 31 path
  files after it, which `check:shards` verifies.
- Lesson ids did not move: all fifteen keep the `GE-C05-*` prefix. Sequences
  shifted inside the 188-215 gap left by chapters 4 and 6.

### Counters, re-measured against the merged tree rather than composed

| Measure | Before | After |
|---|---|---|
| German hand-written chapters | 11 | **10** |
| atoms taught | 267 | 297 |
| lessons with measured budgets | 124 | 139 |
| atom-measurement-blind lessons | 26 | 21 |
| chapters over the 12-atom budget | 4 | 5 |
| forward references | 40 | **36** |
| atoms never revisited | 73 | 81 |
| reinforcement window misses | 563 | 645 |
| full paradigm grids (corpus) | 22 | **21** |
| `handwritten_parity.py german` | 55 | 51 |

Forward references fell by four: teaching *er*, *wir*, *ihr*, *wo*, *was* and
*in* retired previews earlier chapters had been making. The direction was read
off the tree, not assumed — this number has moved both ways across the four
German chapters retired so far.

German stays pinned at **zero** cross-chapter prose references.

### The full-grid count went down, and that is the gate working

`info-dump.test.ts` pinned `fullParadigmGrids` at 22 and named
**`GE-C05-wohnen`** as one of two canonical fixtures — a lesson presenting a
complete conjugation in a single table. Splitting that grid across the lessons
that teach it removed the last German example: no German lesson now presents a
full paradigm in one table, and the corpus count is 21.

That six-row table was the exact shape the info-dump gate exists to flag, so the
count falling is the gate working rather than the detector breaking. The named
fixture that protects the detector is `FR-C05-parler` alone now, and the German
assertion was **inverted to `toBe(false)`** with the reason written beside it
rather than deleted — so if anything ever reintroduces a German full grid, the
test still fails loudly. French chapter 5 has the same grid, so whoever retires
it faces this decision too.

### Deliberately not carried over

*Ich mache einen Kuchen* would have introduced **two** untaught words (*einen*,
*Kuchen*) to illustrate a verb that already has an example. The *lernen*
etymology's detour through Gothic *lais* and a shoemaker's *last* is weight
without payoff next to *learn*/*lore*, which carries the same point. Also
dropped: *won't* as a cousin of *wohnen*, archaic *won*/*wone*, French
*thiois*, and the reconstructed `*þeudō` behind *diutisc*. Everything else the
hand-written chapter taught survives — 50 of the 56 claims read across, and the
six above are the six.

