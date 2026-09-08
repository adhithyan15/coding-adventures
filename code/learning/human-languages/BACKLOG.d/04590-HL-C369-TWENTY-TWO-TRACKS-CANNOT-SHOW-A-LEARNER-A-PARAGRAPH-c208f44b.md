## HL-C369 — Twenty-two tracks cannot show a learner a paragraph

Measured across all 23 tracks: **three lessons in 6,574 carry `type: reading`**,
and Spanish — the deepest track, 1,155 lessons and 417 chapters — had none. Two
of the three are Japanese body-map lessons and one is a Punjabi assembly drill;
not one of them contains a connected passage.

The cause was structural rather than an authoring oversight. `comprehension` had
been a member of `LessonBlockType` since schema v2 with **nothing on either side
of it**: no authored heading classified to it, and no renderer consumed it. A
passage could only reach a book by being labelled `You'll want to know` and
declared an input block, which is why the three nominal reading lessons contain
no passage. That is the same trap the classifier's own comments call out for
`the letters in this word` and `across the family`.

Spanish now has three passages (47, 47 and 61 words). This entry records what
remains.

### The exam shape is known exactly, and nothing checks the corpus against it

`spanish/task-shapes/a1.json` specifies the DELE A1 reading paper in full — four
parts, 25 items, 45 minutes — including the length of text a candidate must read:

| part | items | stimulus |
|---|---:|---|
| `reading-very-short-notices` | 6 | 20–30 words |
| `reading-public-information-match` | 6 | 20–30 words |
| `reading-personal-message` | 5 | **150–175 words** |
| `reading-everyday-information` | 8 | **175–210 words** |

**The longest connected passage anywhere in the corpus is 61 words.** The two
short-notice parts are within reach today; the two long parts need roughly three
times the longest text the curriculum has ever produced.

Nothing measures this. `task-shapes.ts` parses and validates `stimulusLength`,
but no gate compares it to anything the curriculum contains, so a track can sit
at full content coverage while its longest text is a third of what the paper
puts in front of the candidate. **A gate that measures each track's longest
passage against its own declared exam stimulus is the smallest change that makes
the reading goal checkable** — the same argument that made the exam inventories
worth writing.

Note on a claim worth not repeating: the exam INVENTORIES
(`exam-inventory-*-a1.json`) hold no reading points, but that is correct by
design and not the gap. They restate PCIC, which publishes *content* inventories
— grammar, functions, notions, orthography — and has no reading inventory to
restate. Exam readiness lives in the assessment contract and task shapes
instead, and reading is fully specified there.

### Twenty-two tracks still have no passage

The renderer now supports one in any track at no cost, since the box is inline
and needs no `preamble.tex` edit. The blocker is per-track: a passage may only
use words that track has already taught, so each must be built against its own
vocabulary. Connectives of the `luego` kind and a stock of concrete nouns are
the practical prerequisite, which several tracks do not yet have.

Note for whoever picks this up: a reading skill introduced once **revokes the
track's level**. `ES-SKILL-CONNECTED-READING` introduced in a single lesson
dropped Spanish from A1 to pre-A1 on the reinforcement criterion, because an
atom revisited fewer than twice is not taught. Reading arrives in threes or it
does not arrive.
