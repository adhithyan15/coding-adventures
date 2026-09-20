## Unreleased — the script ladder moves to chapter one, one character per lesson

The last tranche taught nine more characters and the closure figure barely
moved, because **closure is measured in READING ORDER**: a character taught in
chapter 20 cannot retire a violation in chapter 2, and chapters 1–13 were where
the violations lived. The blocker on fixing that is gone (`bookhashes.test.ts`
no longer pins per-chapter lesson counts), so the ladder was rebuilt and moved.

Measured with `report-cli` and `measureScriptClosure` before and after:

| | before | after |
|---|---|---|
| script lessons | 23 | 48 |
| characters taught by a script lesson | 44 | 48 |
| characters SHOWN but never taught | 9 | **5** |
| lessons decoding an untaught character | **46** | **31** |
| headwords in Devanagari with no romanization | 4 | **0** |
| first script lesson | chapter 8 | **chapter 1** |

Corpus-wide the closure figure falls 576 → 561; every one of those fifteen is
Sanskrit.

### What was resequenced

All 23 existing segments were rewritten and re-placed, and 25 new ones
(`SA-S200`–`SA-S224`) were authored, giving 48 in total — one for each
Devanagari character the corpus can honestly teach. The order is no longer
"whatever was authored next"; it is **scheduled against the demand curve**, so a
character lands before the first lesson that asks the reader to decode it,
subject to two rules that are checked mechanically rather than trusted:

1. **Exactly one new character per segment.** Not "one in the title" — one in
   the whole lesson, headword and body together, verified by walking the corpus
   in reading order and diffing the taught set. That check found four segments
   smuggling extra characters in through prose nobody had looked at: a component
   description that names another letter (`आ` is described as built on `अ`), a
   stroke note that does the same, and an anchor word's gloss that quoted a
   whole untaught sentence. Every string this tranche interpolates — component,
   stroke, citation, gloss — is now filtered against the characters the reader
   actually has, and an untaught one is transliterated or dropped rather than
   printed. The old `SA-S01-letter-ma` credited **fifteen** characters at once.
2. **Never a character with no word behind it.** A segment may only teach a
   character that some ALREADY-TAUGHT headword contains, so the lesson's
   "you already say these" list is true rather than decorative. Where the anchor
   word's other shapes are not yet taught, the word is printed romanized and the
   character stands alone — the ladder earns the right to print whole words as
   it climbs.

Placement respects HL11 §4's `minLessonsBetweenScriptSegments: 2`: at least two
content lessons sit between any two segments, so the script never becomes the
course. That cadence, not authoring effort, is what bounds the result — a
segment every second lesson leaves 28 slots before chapter 14 against roughly 40
characters of demand, which is why 31 rather than 0.

Chapters 1–5 are handwritten and protected from generation, so their fifteen new
segments were rendered through the book renderer itself and spliced in with
`% canonical-insertion:` markers, and the handwritten coverage ledgers now
declare them. The Sanskrit book compiles clean under XeLaTeX: 430 pages, zero
missing characters.

### Romanization

`धन्यवादः`, `मम नाम … अस्ति`, `तव नाम किम्?` and `भवान् कथम् अस्ति?` were the
last four Devanagari headwords carrying no romanization, which made them
load-bearing script rather than exposure. They have one now, and the track's
count is zero.

### What this tranche did NOT do, and why

- **ऋ and ङ are still untaught.** Neither has an entry in
  `data/scripts/devanagari.json`, so neither has a cited stroke order or a
  component description. Five of the 31 remaining violations are `ऋ` alone. A
  lesson could be written that looked exactly like the other 48 — and a reader
  cannot tell an invented pen path from an attested one. No source, no lesson.
- **ौ, ई, ँ, घ have no headword anywhere in the track.** `ौ` keeps its segment
  (it is used in running text and is built from two shapes the reader has), and
  it says plainly that no word has shown it yet. The other three need vocabulary
  scheduled before a segment can honestly anchor them. NOTE: the earlier backlog
  claim that `इ ई घ ड ँ ू ◌ै` all lack a headword is now **wrong** — `इ`
  (इदानीम्), `ड` (क्रीडति), `ू` (सूनुः) and `ै` (वैद्यः) each have one, and all
  four are taught here. The accurate list is `ौ ई ँ घ ऋ`.
- **Chapter 7 crosses the 12-atom chapter budget** (12 → 15) because three
  segments land there and it was already exactly at budget; chapter 6 was over
  before this tranche (15) and is now 16. Both numbers are report-only. The
  honest alternative was to push `च क ख` later, which costs closure — the thing
  this tranche was asked to move. Recorded rather than routed around.
- **The driving edition pays for this.** Sanskrit's chapter-prefix reach falls
  240 → 179 lessons and its drivable share 91% → 83%, because a script segment
  needs eyes and now sits in the middle of early chapters instead of at the end
  of late ones. HL08's placement measurement predicted exactly this and is why
  the earlier segments were put last in their chapters; moving them earlier is
  the directive, and the cost is stated rather than buried. **No chapter became
  unstartable** — every segment still has at least two content lessons in front
  of it, so the corpus's `unstartableChapters` count is unchanged at 130 and
  none of them is Sanskrit.
- **Five more chapter payoffs fall below the representativeness floor** (80 →
  85), because chapters 1–5 now carry typed atoms while their legacy schema-v1
  payoffs assess nothing. That debt is the schema migration's, not the ladder's,
  and inventing an assessment to hide it would be worse.

