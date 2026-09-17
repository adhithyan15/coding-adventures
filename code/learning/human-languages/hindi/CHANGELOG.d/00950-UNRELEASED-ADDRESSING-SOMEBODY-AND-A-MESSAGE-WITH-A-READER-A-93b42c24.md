## Unreleased — addressing somebody, and a message with a reader at the top

Chapter 105 closes `HI-A1-S-03` and `HI-A1-S-05`. **They are one piece of work,
and neither note said so.**

| metric | before → after |
|---|---|
| exam-point coverage | 237/282 → **239/282 (85%)** |
| atoms taught | 542 → 545 |
| measurable lessons | 484 → 489 |
| writing-practice lessons | 124 → 126 |
| `forward-language` | unchanged (22) |
| `scriptClosureViolations` | unchanged (20) |
| `payoff-surprise` | unchanged (13) |
| `atomsNeverRevisited` | unchanged (33) |
| `durationViolations` | unchanged (0) |
| reinforcement-window misses | 1284 → 1298 |

### The two points were coupled and read as separate

`HI-A1-S-05` wants *a connected two-sentence message for a named reader*, and
its note credited `HI-W06-two-sentence-card` and `HI-W06-two-sentence-no-model`
with teaching it. Those lessons do produce two connected sentences. **They
produce a card.** No reader is named anywhere in either of them, while the exam
task they are measured against is addressed to Reema or Mohan and pays 10 of
that task's 60 points for the named reader — which is `HI-A1-S-03`.

So half of S-05 was blocked on S-03, and neither note pointed at the other. The
chapter closes both, in that order.

### What the chapter teaches

**जी moves from behind a yes to behind a name.** The learner has had it since
chapter 7 in `जी हाँ`; putting it after a name turns somebody being *talked
about* into somebody being *talked to*. Hindi has no separate vocative ending —
**the position is the grammar**, and nothing in the following sentence changes.
That last part is worth its own paragraph in a track where endings agree, vowels
shorten and verbs bend for the speaker.

**प्रिय does the same job on the other side of the name, for the page.** One
word leads and one follows, which is the kind of thing a reader gets backwards
once and then remembers.

**A message is three parts stacked down a page**: the line naming the reader,
the sentences underneath, the writer's name at the foot. Every word of the model
— `प्रिय मीरा,` / `नमस्ते। कल मिलते हैं।` / `— अरुण` — has been written before.
What is new is the arrangement, and the one punctuation fact that comes with it:
the opening takes a comma rather than a **।**, because it is not a sentence.

### Three things the draft got wrong, and what caught each

**A sixth lesson was misplaced by eighty-three lessons.** The draft taught
**सुनिए** as the idiomatic attention-getter, and `forward-language` went 22 → 23:
`HI-C85-sintesis-suchna` already prints **कृपया सुनिए** at chapter 93, where the
reader builds it productively from the respectful **-iye** ending that chapter
teaches. Introducing it as a lexical item twelve chapters later is backwards, so
**the lesson was withdrawn rather than reworded** and `HL-C389` carries the sense
to a home beside chapter 93. It was also the chapter's only script-closure debt:
spelling the ending out in prose put the independent vowel **इ** in front of a
reader nobody has drawn it for.

**The writing lesson was one lesson and had to be two**, for two unrelated
reasons the snapshot diff surfaced together. As one lesson it computed **301
seconds against a 300-second ceiling**. And its atom was introduced in the
chapter's *last* lesson, so nothing could ever revisit it and
`atomsNeverRevisited` went 33 → 34. Splitting it fixes both at once and is
better teaching besides: `HI-C97-sandesh` shows the shape with a model in front
of the reader, and `HI-C97-sintesis-sandesh` produces the whole message from a
cue alone.

**None of the three was visible in any gate.** All twelve passed on the draft.

### The one metric that moved the wrong way

Reinforcement-window misses go 1284 → 1298. That is the structural cost of
adding a chapter at the end of a corpus: atoms introduced in the last chapter
have no later lessons to be revisited by, and five new lessons that do not
revisit an older atom widen every open window they fall inside. Chapter 104 paid
the same toll (1261 → 1284 for eight atoms). It is recorded rather than
explained away.

### Verification

- 145 files / 2083 tests passed and 1 skipped in `human-language-data`, on a
  quiescent tree
- all twelve `check:*` gates
- `validate` — 0 errors
- `check-book-compile.sh hindi` under XeLaTeX — compiles
- `/security-review` — passed

43 exam points remain open. Coverage means the teaching exists, not that a
reader scores. No track has had a single exam item graded against it.
