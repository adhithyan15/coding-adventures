## German chapter 16 becomes three chapters, and leaves the hand-written set

`ch16-sein.tex` — the top of the German hand-written range — is now generated,
as **chapters 16, 17 and 18**. German's hand-written chapters: **12 -> 11**.
Old chapters 17-31 renumbered to 19-33.

### Sizing it four ways, and only the fourth was right

- `handwritten_parity.py` scored it at a gap of **8 blocks**.
- `grep -l '^chapter: 16$' lessons/*.md` said **3 lessons**, and the `.tex`
  rendered **2 sections**. No hidden writing lessons this time.
- Counting the German words it teaches gave six new ones: *sein*, *müde*,
  *kommen*, *fahren*, *werden*, *bleiben*.

None of those is the real size, because chapter 16's cost is **grammar, not
vocabulary**. It carries two paradigms — the present of *sein* and its past —
and a cell is one filled slot, not a table. Six present cells plus four past
cells plus six lexical, sound and rule atoms is **twenty-four atoms**, against a
`maxNewAtomsPerChapter` of twelve.

Length is never a cost here, so it became **three chapters of 9, 7 and 8 atoms**
rather than one chapter at twice the ceiling. Three lessons became **twenty-six**.

### The paradigm grid appears only as a recap

HL10's rule is that no paradigm table may be printed until every cell in it has
been taught individually, at which point the table is a recap rather than an
introduction. So `sein` is met one form per lesson — *bin*, *bist*, *ist*,
*sind*, *seid*, *sind* — and the grid the hand-written chapter opened with now
sits in `GE-C16-praesens-practice`, at the end, where it is the ninth thing the
reader sees rather than the first.

The past does the same across chapter 17, and by chapter 18 the
*ich bin gegangen* table is a recap twice over: every cell of *sein*'s present is
owned, and the participle does not inflect.

### Lesson ids did not move

All twenty-six lessons keep the `GE-C16-*` prefix and live across chapters 16,
17 and 18 — the Spanish convention, where `ES-C03-*` lessons sit in chapters 4,
5 and 6. An id names the chapter a lesson was written for; chapter numbers move
and ids do not.

### German joins the tracks whose chapters move

Fifteen chapters renumbered at once, and **36 of German's 65 cross-chapter prose
references pointed into that range** — every one of them would have rotted on
this single commit. `chapter-references.test.ts` says what to do: name the thing,
never the number. All **65 -> 0**, and German is now pinned at zero beside
Spanish and French. The pass used "the food lesson", "the doing verbs", "the
*Hand* table", "your first verbs", "the eszett rule", and for the purely
decorative ones the thing itself: "**hören** showed", "**Hund** showed".

### Counters, re-measured against the tree rather than derived

| Measure | Before | After |
|---|---|---|
| German hand-written chapters | 12 | **11** |
| `handwritten_parity.py german` | 63 | **55** blocks at risk |
| lessons (schema-v2) | 98 | 124 |
| atoms taught | 243 | 267 |
| atom-measurement-blind lessons | 29 | 26 |
| chapters over the 12-atom budget | 4 | 4 |
| culture claims | 12 | 14 |
| atoms never revisited | 75 | 73 |
| forward references | 39 | **40** |
| cross-chapter prose references | 65 | **0** |

Forward references went **up** by one, and the direction is the point. Teaching
*ich bin* made `GE-C14-alter`'s existing *ich bin zwanzig Jahre alt* visible as a
preview for the first time — the gate cannot see a reference to a word no lesson
teaches. Two rose that way, one fell (`nein` was cut from a dialogue that did not
need it), and one is deliberate: *die eingeschlafene Katze* is the hand-written
chapter's own example of a participle taking an adjective ending, and it is kept
with a note rather than swapped for a duller noun.

### Two defects the gates caught

1. **A capital eszett's cousin.** `GE-C16-muede` wrote the stress as *MÜ-duh*.
   Capital **Ü** is in neither `core/main-font-charset.json` nor `book.ts`'s
   escape map — free while the chapter was hand-written, a hard `glyph-coverage`
   failure once generated. Lowercase *mü-duh* with the stress stated in words.
2. **Two dialogues read as paradigms.** `info-dump.ts` calls a table with three
   or more person-labelled first cells a paradigm table, and a German exchange
   whose lines open *Ich…*, *Du…*, *Wir…* looks exactly like one. Rewritten so
   the speakers open with something other than a pronoun; corpus `paradigmTables`
   back to 95 and `lessonsWithFindings` to 121, both unchanged from before.

Also fixed on the way past: `GE-C16-ich-war` said "look at the shape of it",
which is a `sight-cue`, and that one phrase cost chapter 17 its hands-free start.
It says "hear" now, and all three chapters open drivable.

### Verification

124/124 test files, 1739 tests; every `check:*` gate; language-ladder 39/39
files, 442 tests; the German book compiles under XeLaTeX with zero errors and
zero overfull or underfull boxes (279 pages), and chapters 16, 17 and 18 were
read on the page. All 70 teaching claims in `ch16-sein.tex` were checked across
into the new lessons.

