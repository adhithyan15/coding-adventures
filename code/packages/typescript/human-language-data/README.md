# @coding-adventures/human-language-data

The machine-readable bridge from the **Human Languages** curriculum (spec
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md)) to the
cross-language dataset that downstream tools — the Engram practice deck
([`HL02`](../../../specs/HL02-companion-practice-app.md)) and anything later —
consume. It implements [`HL01`](../../../specs/HL01-concept-taxonomy-and-data-layer.md).

## What it does

The curriculum is a pile of per-language Markdown lessons. This package parses
their frontmatter and lossless typed body blocks, joins them through the canonical **concept taxonomy**
(`concepts/taxonomy.json`), and exposes the result as a queryable dataset —
plus a **validator** that keeps the lessons and the taxonomy from drifting apart.
It also produces the versioned migration-gap report published with every book
bundle and deterministically renders configured LaTeX chapters from that same
lesson AST.

The Human Languages index is derived too. `npm run generate:progress` rewrites
only the marked track table in `code/learning/human-languages/README.md`; CI runs
`npm run check:progress` so registry additions, curriculum growth, and generated
book chapters cannot leave its counts stale.

```
lessons/*.md frontmatter  ─┐
<track>/curriculum.json    ─┼─►  Dataset + local realization paths
concepts/taxonomy.json     ─┤         + validate() / validateCurriculum()
data/scripts/*.json        ─┘         + independent frontier planning
```

The core is the **concept**: a language-independent idea (`GREETING-HELLO`). Each
language's word for it is a **realization** (Spanish *hola*, Telugu *నమస్కారం*).
That join is what lets a study app say "here is *hello* in every language you're
learning."

## Usage

```ts
import {
  languagesForConcept,
  loadEverything,
  mixedCurriculumFrontier,
  compileLessonActivities,
  validate,
  validateCurriculum,
} from "@coding-adventures/human-language-data";

const { curricula, dataset, lessons, registry, scripts, spine, taxonomy } = loadEverything();

// Typed activities compile directly from block metadata, never from prose.
const activities = compileLessonActivities(lessons[0].blocks);

// The cross-language join:
languagesForConcept(dataset, "GREETING-HELLO");
//  → [{ language: "spanish", headword: "hola", … }, { language: "telugu", … }, …]

// The consistency gate:
const issues = [
  ...validate({ taxonomy, lessons, scripts }),
  ...validateCurriculum({ curricula, lessons, registry, spine, taxonomy }),
];

// Each language advances on its own prerequisite-closed path. Only frontiers
// that are simultaneously ready at the same shared node are grouped.
mixedCurriculumFrontier(
  curricula,
  ["persian", "urdu"],
  new Map([
    ["persian", new Set(["FA-C01-salaam"])],
    ["urdu", new Set()],
  ]),
);
```

### Exam task shapes (HL18)

`<track>/task-shapes/<level>.json` records the sourced performance target behind
an exam-ready claim: what the learner reads or hears, what they must write or
say, timing, interaction, replay, scoring and aids for all four skills. Unknown
source measurements remain explicit rather than being guessed.

```ts
import {
  buildTaskShapeBacklog,
  listTaskShapeInventories,
  loadLanguageRegistry,
  loadTaskShapeInventory,
} from "@coding-adventures/human-language-data";

const germanA1 = loadTaskShapeInventory("german", "A1");
const present = listTaskShapeInventories();
const missing = buildTaskShapeBacklog(
  loadLanguageRegistry().languages.map((track) => track.id),
  present,
);
```

The first inventory is official Goethe German A1. It is a target for later
five-minute lesson decomposition and mocks, not a claim that the current German
book is pass-ready.

### Chapter capabilities (HL05)

A chapter used to be nothing but an integer on each lesson, so nothing could check
that finishing one left the reader able to do anything. `<track>/chapters.json` is
that missing promise — a first-person `canDo` plus a payoff the reader can deploy
immediately — and `core/chapter-policy.json` holds the thresholds that judge it.

```ts
import { loadTrackChapters, loadChapterPolicy } from "@coding-adventures/human-language-data";

const ledgers = loadTrackChapters();   // tracks WITHOUT a ledger are skipped, not defaulted
const policy = loadChapterPolicy();    // payoff share + HL08 gentle-ramp budgets
```

The skip is deliberate. "Not yet authored" and "authored and empty" are different
kinds of debt, and defaulting the first into the second would erase exactly what the
gap report exists to measure. Ledgers are **authored intent** — unlike
`curriculum.json`'s `omits`/`relocates`, which are recomputed caches, no validator
may rewrite them.

### Productive pattern lessons (HL05)

`type: pattern` turns a reusable frame into canonical book-and-app data. A pattern
introduces exactly one `*-PATTERN-*` atom, declares ordered slots whose fillers are
already present in `requires.knowledge`, and gives the reader at least three distinct
guided-production instantiations. The tiny frontmatter parser represents the ordered
map as typed `patternSlots` on `ParsedLesson`:

```yaml
type: pattern
requires:
  knowledge: [ES-LEX-COMER, ES-LEX-BEBER, ES-LEX-CAFE]
introduces:
  knowledge: [ES-PATTERN-ER-FUTURE-SINGULAR]
slots:
  infinitive: [ES-LEX-COMER, ES-LEX-BEBER]
  object: [ES-LEX-CAFE]
```

The three chapter gates reject an extra introduced atom, a missing or non-list slot,
an out-of-closure filler, or fewer than three distinct guided productions. The first canonical
realization is Spanish `ES-C17-comer-futuro`: known *comer*, *beber*, and *café* fill
the new singular future frame without smuggling in vocabulary.

### Modality and the drivable course (HL08)

[`HL08`](../../../specs/HL08-modality-gentle-ramp-and-the-drivable-course.md) asks a
concrete question: **how much of this can I learn in the car?** `modality.ts` answers
it for every lesson and every chapter.

```ts
import { summarizeModality, deriveLessonModality, loadEverything } from "@coding-adventures/human-language-data";

const { lessons } = loadEverything();
const modality = summarizeModality(lessons);
modality.drivablePercent;              // 84 — the share learnable by ear alone

modality.drivablePercent;              // 65 — the share a hands-free view can deliver
modality.tracks[0].chapters[0];        // { drivablePrefix: 5, firstNonVoiceLesson: "…", … }
deriveLessonModality(lessons[0]).reasons; // ["wide-table"] — why it needs eyes
```

### Attaining a level, versus touching one (HL09 §3.1)

```ts
import { runLevelGate } from "@coding-adventures/human-language-data";

const gate = runLevelGate({ lessons, levels, curricula, spine, ramp, continuity });
gate.tracks.find((t) => t.language === "spanish");
// { touches: "A2", attained: null, inProgressAt: "pre-A1", vocabulary: 113,
//   blockers: [{ criterion: "vocabulary", shortfall: 256,
//               detail: "teaches 44 distinct headwords at or below pre-A1, against 300" }, …] }
```

`touches` is the highest level any lesson **sits at** — one lesson pointing at one A2
node is enough to move it. `attained` is the highest level where all four §3.1 criteria
hold, here and below: spine nodes realized, cumulative vocabulary met, no lesson over
the atom budget, every atom revisited twice. Every criterion is scoped **at or below
the level** — Spanish teaches 113 headwords in total but only **44** at or below
pre-A1, and it is the 44 the gate judges.

**Zero of 23 tracks have attained even pre-A1.** That gap between the two numbers is
what let "Spanish reaches A2" stand on fourteen present-tense lessons.

### What to do next — the computed backlog (HL15)

```bash
npm run plan              # the ordered queue, text
npm run plan -- --format json --head 5
npm run plan -- --ceiling A1     # just the first rung
```

The gate above says what is *wrong*. This says what to *do* about it, and it is a
pure function of the same measurements rather than a list somebody maintains:

```text
23 tracks, 0 done; 146 enumerable item(s) today, ~10,791 projected to C2

    1. [pre-A1] assessment-contract — marwadi
       require independent four-skill passes, a writing ramp, and timed full mocks
    2. [pre-A1] assessment-contract — japanese
       require independent four-skill passes, a writing ramp, and timed full mocks
    3. [pre-A1] assessment-contract — chinese
       require independent four-skill passes, a writing ramp, and timed full mocks
```

Three ordering rules, all mechanical. **The floor is universal** — every pre-A1 item
outranks every A1 item in any track, because a track that climbs while its floor is
missing has built a cliff with upper-level lessons on top. **Family priority** then
decides what a track's next action is: the complete pass-ready assessment contract
first, then its sourced four-skill task shape, external/project content inventory,
script closure, exam points, and vocabulary. And the
queue **rotates across tracks**, furthest-behind first, so every language moves once
before any language moves twice.

Two corrections are baked in and worth knowing about, because both looked right:
a flat sort produced a head of 22 consecutive research tasks and no content, and an
inventory for a rung a track has not reached was outranking the floor it is standing
on. See `code/specs/HL15-the-completion-plan.md` §4.

Three of the eight families report `null` rather than a number in the projection.
That means **not projectable** — reinforcement debt at B1 is a function of lessons
nobody has written — which is a different fact from zero.

### Continuity — does the course have a memory of itself? (HL09)

The ramp budgets measure how big each **step** is. This measures whether the steps
hold together.

```ts
import { measureContinuity, loadEverything } from "@coding-adventures/human-language-data";

const { lessons } = loadEverything();
const c = measureContinuity(lessons);

c.summary.lessonsWithoutSequence;   // 515 — no declared reading order at all
c.summary.atomsNeverRevisited;      // 745 of 1599 (47%) taught once, never again
c.summary.missedByWindow;           // { R1: 778, R2: 1159, R3: 711, R4: 137 }
c.summary.forwardReferences;        // 504 uses of material a later lesson teaches
```

Order comes first: 515 lessons carry no `sequence`, so their order exists only in
hand-typed LaTeX (French: **64 of 73**). A ramp whose order is unknown cannot be
verified, so every other number is provisional until that reaches zero.

Spanish is the worked example. Declaring the real order for 50 of its lessons cut
its forward prerequisites from **31 to 5** — 26 of the 31 were never real, just
artifacts of an alphabetical fallback sorting `beber` before `comer`.

Reinforcement reads `practises.knowledge`, **never `reviews_of`** — which 144 of
Spanish's 146 lessons set and which cannot close a window, because it names lesson
ids while atoms live in another namespace.

Forward references carry their own evidence: a word is reported only when a *later
lesson's headword* teaches it. That reproduces what a human reviewer found by
reading — `ES-C07-beber` says *"Como pan y bebo agua"* while `pan` and `agua` are
chapter 26.

### The two ramps (HL08)

Gentleness has **two** curves, and until HL-C18C only one of them was counted.

```ts
import { measureRamp, loadChapterPolicy, loadEverything } from "@coding-adventures/human-language-data";

const { lessons } = loadEverything();
const ramp = measureRamp(lessons, loadChapterPolicy());

ramp.summary.lessonViolations;          // 40 — lessons above 3 new ATOMS (meaning)
ramp.script.summary.lessonViolations;   // 61 — lessons above 3 new GLYPHS (decoding)
ramp.script.summary.steepestLesson;     // HI-W01-shirorekha-na-ma: 1 atom, 12 glyphs
ramp.script.summary.systemViolations;   // 5 — lessons opening >1 writing system at once
```

The two come apart badly. `HI-W01-shirorekha-na-ma` declares **one** knowledge atom and
puts **twelve** new Devanagari glyphs in front of the reader; it passes the atom budget
comfortably. 38 of the 61 over-budget lessons declare *zero* atoms, so they read as
maximally gentle while teaching up to a dozen new shapes.

**Target script and cousin script are counted separately, and only the first is charged.**
A Kannada Chapter 1 lesson that shows the same word in Devanagari, Tamil, Telugu and
Malayalam looks like a 34-glyph cliff if you add them together — its actual Kannada load is
7. Sister-script material is context for a reader who already knows a relative; English is
the only requirement for any book, so that layer is reported and never penalised.

Both are **report-only**: the debt predates the measurement.

Three channels, each naming what the learner must have available:

| Value | Sign | Meaning |
|---|---|---|
| `voice` | 🚗 | learnable by ear alone — a voice assistant can teach it while you drive |
| `sight` | 👁 | needs eyes — letter shapes, figures, or a table that cannot be read aloud |
| `pen` | ✍ | needs a hand — handwriting formation and practice |

Modality is **monotonic**: `pen` implies `sight`, so a pen lesson requires both. A
chapter's modality is the union of its lessons'.

**Modality is derived, never read off `skills:`.** That field records what a lesson
*develops*, not what it *requires*: 501 of the 531 schema-v2 lessons declare
`[listening, speaking, reading]`, yet *hola* is perfectly learnable by ear. Deriving
from `skills` would have stamped roughly 95% of the corpus "needs eyes" and made the
drivable course an empty promise. The derivation reads lesson type and block
structure instead:

1. `type: writing` → `pen`;
2. otherwise a `script` block, a sight cue, or a table wider than the configured
   linearisable width → `sight`;
3. otherwise → `voice`.

`maxLinearisableTableColumns` defaults to **3**, and modality does not decide that on
its own — it asks the *same* lineariser the narration export uses (`speech.ts`), so
"drivable" can never mean something the export cannot actually deliver. A table is
`sight` when the lineariser refuses it, whether because it is too wide, its rows are
ragged, or it has a heading row with nothing under it. The knob lives in
`core/chapter-policy.json`; the pre-lineariser behaviour is still one argument away
(`summarizeModality(lessons, { maxLinearisableTableColumns: 0 })`).

Raising it from 0 to 3 moved the corpus from **63% drivable to 84%** (694 → 925 of
1,096 lessons). See the narration section below for why three columns is the honest
line and four is not.

A **chapter's drivable prefix** is how many of its lessons, in authored `sequence`
order, have a `voice` core before the first that does not — deliberately not "how many
voice lessons does it contain", because chapters are prerequisite-ordered and a voice
lesson sitting behind a sight one is not reachable in the car.

Every downloadable book projects that same result at the start of each chapter.
`generate:books` writes one byte-gated `chapter-modalities.tex` file per track with
font-independent car, eye, and pen signs, full printed-lesson counts, and the
core-based hands-free prefix. Generated and protected handwritten chapters both call
the projection immediately after their title and label, so a reader sees the requirement
before the first lesson and the book cannot drift from the app or narration model.

#### One lesson, two modalities (HL-C41)

The three rules above give a lesson one answer, which is right for the **book** — it
prints every block and needs one honest sign at the chapter opening. It is wrong for a
lesson that is voice throughout except for a short section teaching the hand to form a
letter met earlier. Under one answer per lesson those two minutes cost all five.

So modality is derived at two scales:

| Field | Reads | Answers |
|---|---|---|
| `modality` | the whole lesson | what the **book** signs |
| `coreModality` | the lesson minus its **detachable** blocks | what a hands-free view can deliver |

```ts
const entry = deriveLessonModality(lesson);
entry.modality;         // "pen"   — the book prints a writing segment
entry.coreModality;     // "voice" — a listener still gets the rest
entry.writingSegments;  // ["Writing: మ — the tick on top"]
entry.blocks[1];        // { type: "writing", modality: "pen", detachable: true, … }
```

A block type is **detachable** when nothing later in the lesson depends on it. Exactly
one is today — `writing` (heading `## Writing: …`), which teaches the *hand*, as
against `script`, which teaches the *eye* and is not detachable. Detachable means "a
non-visual renderer may set this aside", never "optional content": **the book prints
every block, in full.** A future dictation-friendly edition is a separate output view
over the same source, and `coreModality` is the metadata it reads.

`drivablePercent` and the drivable prefix are counted on the core; the
`voice`/`sight`/`pen` counts still describe the book, and `coreVoice` is published
beside them so the two reconcile. An authored `modality:` override caps the core, so
the core is never stronger than the full modality. A lesson that is not `type: writing`
may carry **one** writing segment; more is reported as
`modality-writing-segment-not-separable`.

An author may override with `modality:` in frontmatter; an override that
*contradicts* the derivation additionally requires `modality_reason:`. Unexplained
overrides and unknown values are **reported, never thrown** — `summarizeModality()`
walks the whole corpus and returns every finding at once. This slice ships **no
gates**, per the HL-V01 precedent.

Measured over all 1,096 lessons: 51 `pen`, 7 with `script` blocks, and among the
remaining 1,038, 322 carry a Markdown table — the single largest obstacle to a
hands-free course, and a far more tractable one than the script. With the lineariser
shipped, **925 lessons (84%) are drivable**, up from 694 (63%) when every table meant
eyes. Of the 120 that still need eyes: 65 carry a table of four columns or more, 61
point at something on the page in prose, and 7 have a `script` block. **52 need eyes
for a wide table and nothing else** — that is HL08's table-remediation burn-down list,
and reshaping just those tables would move 52 more lessons into the car.

### The letter ledger — what order to meet the letters (HL11)

The ramp budgets say how fast glyphs may arrive. They say nothing about *which*
glyphs, and for a reader who cannot yet speak the language that is the whole
question.

```ts
import { loadEverything, loadChapterPolicy, validateLetterLedger, summarizeLetterLedger }
  from "@coding-adventures/human-language-data";

const { lessons, letterLedgers } = loadEverything();
const ledger = letterLedgers.find((l) => l.script === "tamil")!;

summarizeLetterLedger(ledger);
// { script: "tamil", tracks: ["tamil"], positions: 24, openingWords: 30,
//   firstWritableWord: "…", firstWritablePosition: 2,
//   writableAfter: [{ position: 8, words: 2 }, { position: 16, words: 11 },
//                   { position: 24, words: 18 }] }

validateLetterLedger(ledger, lessons, { unspentWindow: 6 });  // []
```

Ordering letters by the words they complete reaches **நன்றி at Tamil's tenth
glyph and வணக்கம் at its eleventh**, and नमस्ते at Devanagari's twelfth. The same
walk in traditional recitation order completes **zero** words after twelve
glyphs — that gap is the reason the ledger exists.

`firstWritableWord` is deliberately a *word* and not a letter count. Twenty
taught letters is not an achievement a reader can feel; writing *thank you* is.
The measurement is of the payoff, not the effort — the same reason HL05 measures
a chapter by what the reader can do at the end of it.

The ledgers are **authored intent** (`data/scripts/<script>-ledger.json`), like
`chapters.json`: proposed by `data/scripts/propose_letter_ledger.py`, reviewed,
and committed. `validateLetterLedger` may check one and may never rewrite it. It
asks six questions, each with an answer that is invisible by eye — contiguous
positions, glyphs really belonging to the named script, no vowel sign before a
base letter, families sitting together, every claimed unlock naming a lesson
that exists, and no letter that unlocks nothing for a whole window (the Root
Ledger's rule applied to glyphs).

The fifth is the one that earns its keep: a ledger and a curriculum drift apart
silently, because the ledger keeps asserting a payoff long after the lesson
delivering it was renamed, and nothing else would notice.

Note that `loadScripts` deliberately skips `*-ledger.json`. Both files live in
`data/scripts` and both carry the same `script` key, so reading them into one
map would have had one silently overwrite the other, with the winner decided by
filename sort order.

### Script closure — was the reader ever taught these letters? (HL11)

The glyph budget above measures **pace**. It caps how fast new glyphs arrive and
it caught real spikes. But a track satisfies it perfectly while teaching no
letters at all, and that is what most non-Latin tracks do.

```ts
import { loadEverything, measureScriptClosure } from "@coding-adventures/human-language-data";

const { lessons } = loadEverything();
const closure = measureScriptClosure(lessons);

closure.summary.violations;                    // 932
closure.summary.tracksTeachingNothing;         // 12 of 16 non-Latin tracks
closure.summary.headwordsWithoutRomanization;  // 489
closure.summary.exposureExemptedGlyphs;        // 1997 — what the rule removed
closure.unknownScriptTracks;                   // [] — unmeasured, never "clean"
closure.violations[0];                         // { lessonId: "TE-C16-nelalu", count: 30, … }
```

Put the two numbers beside each other and the gap is the argument for the whole
measurement:

| | flagged lessons |
|---|---:|
| script **ramp** — glyphs arriving faster than the budget | **61** |
| script **closure** — glyphs arriving untaught | **932** |

**Exposure is what keeps this from being absurd.** A word the reader is merely
*shown* — the headword printed beside its romanization, so the eye starts
recognising a shape long before the hand can make it — is not something they were
asked to read. HL11 calls that exposure; it is counted, reported, and never
required. That is what lets a Tamil course open on வணக்கம் while still promising
the reader is never asked to decode something untaught.

The distinction has to be mechanical or it is worthless, so it is drawn where the
corpus already records the answer: **a headword is exposure when the lesson
declares a `romanization`.** Which is also why it is the right rule — it names
its own remediation. Adding a romanization converts a headword from load-bearing
to exposure, and that is a real improvement to the lesson rather than a way of
hiding from the measurement. 489 lessons are one romanization away.

Two numbers watch the exemption, not one. `exposureOnly` counts lessons the
rule flipped to clean — 49. `exposureExemptedGlyphs` counts what it actually
removed, including from lessons that violate anyway — **1,997**. The second is
the one that matters: a lesson reporting five untaught glyphs while fifteen more
were exempted is not a lesson with five problems, and a per-lesson count cannot
see that. It is also the number that would move if an author started laundering
script through the headword once 932 becomes a burn-down target.

A glyph counts as **taught** by any `type: writing` or `delivery: script` lesson
that contains it. That is coarser than naming each letter in frontmatter, and
deliberately so: it credits the corpus with everything it could plausibly be
teaching, which makes the reported debt a *lower bound*.

### The narration export (HL08)

The audio-script output [`HL04`](../../../specs/HL04-shared-spine-and-content-pipeline.md)'s
pipeline diagram has always named. It turns the lesson AST into something an AI voice
assistant can read aloud while the learner drives — HL08's stated purpose, in the
project owner's own words: *"I want to be able to have one of the AI chatbots with
voice capabilities read through and teach me while I am driving."*

```bash
npm run build
npm run generate:narration   # writes <language>/narration/chNN.{txt,json}
npm run check:narration      # byte-for-byte; exits 1 on drift
```

Two views per chapter:

- **`chNN.txt`** — one continuous script. Hand it to any voice assistant with "read
  this to me". Directives appear as bracketed stage directions (`[pause 2 seconds]`,
  `[your turn — say: …]`, `[question — say your answer, then pause 9 seconds]`),
  because that is a form every model already reads as an *instruction to the reader*
  rather than words to pronounce.
- **`chNN.json`** — the same script with its joints intact: typed segments a voice
  agent can act on, so it pauses where the lesson says pause, waits where it says
  `[YOU SAY: …]`, and **scores a spoken answer against the compiled activity contract
  from `activity.ts`** — never against prose.

That last point is the module's governing rule. Lessons contain two similar-looking
things and this export never conflates them:

| In the lesson | Becomes | Scored? |
|---|---|---|
| `[YOU SAY: "hola" — OH-la]` | a `prompt` segment | ❌ — a rehearsal, no answer key |
| `<!-- hl-activity: {…} -->` | an `activity` segment | ✅ — `acceptedResponses` from `compileLessonActivities` |

**Tables are linearised, never dropped.** A two-column word→gloss table becomes
*"नमस्ते means hello"*; a three-column table becomes labelled facts, *"Language:
Telugu. Hello: namaskāram. Source: Sanskrit."* Three columns is where a table stops
being a list of facts a listener can hold and starts being a grid whose meaning lives
in the comparison *across* rows — the corpus's own four-column tables prove it, with an
unlabelled first column that only means something because of where it sits on the
page. At that width the lineariser reads **371 of the corpus's 442 tables (84%)**,
covering 272 of the 340 table-bearing lesson files.

A table it refuses is **spoken, not skipped**: the learner hears its size, its column
headings, and why it needs eyes, and the lesson is marked `sight` so they are warned
before they start. `sight` and `pen` lessons still export in full, opening with a
notice naming what they will need and which sections to leave until they have stopped.

Target-script text carries its `romanization` alongside — *"خداحافظ (khodâ hâfez)"* —
using the whole chapter's headwords, so a lesson can pair a word a neighbouring lesson
introduced. Pairing is whole-word only: the Arabic track teaches ا (*alif*) as its own
lesson, and a substring replace once turned سلام into `سلا (alif)م`.

The export is hash-gated exactly like the generated `.tex`:
`core/generated-narration-hashes.json` records an FNV-1a fingerprint of each chapter's
lesson AST and of the two files generated from it, so a lesson edited without
re-running the exporter fails `--check` instead of leaving a voice assistant
confidently teaching a lesson that no longer exists.

**Out of scope, per HL04 and HL08:** no audio. No TTS, no voice selection, no
recordings. This is a script *for* a voice agent.

remaining 1,038, 308 carry a Markdown table — the single largest obstacle to a
hands-free course, and a far more tractable one than the script. **708 lessons
(65%) are drivable exactly as authored.** No track has yet authored an interspersed
writing segment, so every lesson's core equals its full modality and the two-scale
derivation currently moves no number; the regression test pins that.

### The modality manifest — two editions from one source (HL-C44)

The derivation above is only useful to a *program* if it is a *file*. `core/lesson-modality.json`
is that file: one row per lesson, generated and drift-gated, so the complete book, the
app, and the forthcoming dictation-friendly driving edition can each filter the same
canonical corpus instead of maintaining three copies of it.

```bash
npm run build
npm run generate:modality   # write core/lesson-modality.json
npm run check:modality      # fail (exit 1) if it drifted from the lessons
```

```ts
import { loadModalityManifest, modalityManifestById } from "@coding-adventures/human-language-data";

const manifest = loadModalityManifest();
manifest.summary.drivablePercent;        // 65
manifest.lessons.filter((l) => l.drivable);            // the driving edition's lessons
manifest.tracks[0].chapters[0].drivableLessonIds;      // the prefix, already in order
modalityManifestById(manifest).get("ES-C01-hola");     // a Map, never a plain object
```

Each lesson row carries `id`, `language`, `chapter`, `sequence`, `modality`, `derived`,
`drivable`, `reasons`, and the lesson AST's `sourceHash`; the three override fields
(`authored`, `authoredReason`, `overridden`) appear only on the handful of lessons that
have them. Chapters add the drivable prefix and the ids in it; tracks and a corpus
`summary` roll those up.

Two design decisions are worth knowing before consuming it:

- **`modality` is permanently the strongest channel the lesson needs anywhere.** It is
  the conservative filter. HL-C41 has now added *block-level* modality — a lesson core
  that is `voice` beside a short, separable `pen` segment — and it landed as a new
  optional `coreModality` key beside `modality`, never as a change to it. A consumer
  reads `entry.coreModality ?? entry.modality` and is correct both before and after;
  one that never learns about the new key keeps producing a merely *pessimistic*
  driving edition, which is the safe direction to be wrong in. `features.blockModality`
  in the header says whether a given build carries block data.
- **Nothing is authored.** The manifest is derived, exactly like
  `core/generated-book-hashes.json`. HL08 deliberately refused to put `modality:` in
  1,096 frontmatter files, because 1,096 authored copies of a computed fact are 1,096
  places for it to go stale. `check:modality` runs in CI beside `check:books` so the
  manifest cannot drift from the lessons it describes.

Build the JSON and readable gap reports locally with:

```bash
npm run build
npm run --silent report -- --format json > curriculum-gaps.json
npm run --silent report -- --format text > curriculum-gaps.txt
```

Generate configured book chapters, or verify the committed output is current:

```bash
npm run build
npm run generate:books
npm run check:books
```

Generate configured SVG lesson figures, or verify both their canonical inputs and
committed bytes are current:

```bash
npm run build
npm run generate:figures
npm run check:figures
```

`core/figure-generation.json` binds each figure to one canonical lesson and a safe
`<track>/book/figures/*.svg` target. The first figure kind, `etymology-route`, reads
only the ordered `roots` asserted by that lesson and renders them through
`paint-vm-svg`. `core/generated-figure-hashes.json` fingerprints the figure-driving
source subset and the exact SVG separately, so either a stale claim or an edited
artifact fails `--check`. Generated book chapters rewrite the lesson's `.svg` image
destination to `.pdf`; the books workflow creates that PDF with `rsvg-convert`
before XeLaTeX runs.

Regenerate or verify the registry-ordered track table in the top-level Human
Languages index:

```bash
npm run build
npm run generate:progress
npm run check:progress
```

`core/book-generation.json` declares each generated chapter's language, number,
output path, and script-rendering options. Its title and label come only from that
track's `chapters.json` capability ledger; the config is rejected if it tries to
repeat either field, and a declaration without canonical chapter metadata fails
closed. The generator orders schema-v2 lessons by `sequence`, writes the LaTeX
chapter, and records a stable FNV-1a fingerprint in
`core/generated-book-hashes.json`. The fingerprint covers the chapter's lessons
**and the capability fields the book prints** (`title`, `label`, `canDo`, and
`payoff.summary`), so an
edit to `chapters.json` moves it. It does not cover `payoff.note`, which the book
deliberately does not print — a fingerprint covers what the artifact SHOWS. It
detects drift between book and app inputs; it is not a security hash.
The config's `sourceBaseUrl` gives every lesson a stable canonical URL, so
absolute citations and relative prerequisite/reference links remain live after
the generated PDF is downloaded.
Non-Latin targets also declare `unicodeScript` and `scriptCommand`; the renderer
wraps matching Unicode runs in the book's existing font macro and uses each
lesson's `romanization` for a PDF-bookmark-safe short title.

The duration estimator uses instructional word count, explicit pauses, repeat
cues, prose prompts, authored activity response budgets, and a safety margin. Its effective duration is
the greater of that estimate and the lesson's declared budget. A value of 300
seconds or more is reported as migration debt; the report remains non-blocking
until the existing corpus has been split.

### Architecture — a pure core with a thin fs shell

| Module | Role | Pure? |
|---|---|---|
| `frontmatter.ts` | tiny zero-dep frontmatter reader with one nested-map level | ✅ |
| `parse.ts` | frontmatter + Markdown → typed lesson AST; realizations → `Dataset` | ✅ |
| `activity.ts` | typed block activities → normalized runtime answer contracts | ✅ |
| `hash.ts` | stable canonical lesson serialization and deterministic fingerprints | ✅ |
| `book.ts` | typed lesson AST → LaTeX chapter | ✅ |
| `curriculum.ts` | spine, realization-map, prerequisite, schema-v2 duration/block/knowledge validation | ✅ |
| `plans.ts` | ordered local paths, extension placement, next lessons, and mixed ready frontiers | ✅ |
| `validate.ts` | the round-trip validator (errors fail CI; warnings tolerated) | ✅ |
| `queries.ts` | `allConcepts` / `conceptsByLanguage` / `languagesForConcept` / `coverageByLanguage` | ✅ |
| `modality.ts` | per-lesson channel (voice/sight/pen) and per-chapter drivable prefix | ✅ |
| `speech.ts` | Markdown → speakable words; Markdown tables → spoken utterances or a reasoned refusal | ✅ |
| `narration.ts` | typed lesson AST → narration segments and the continuous voice script | ✅ |
| `modality-manifest.ts` | that derivation as an emittable, filterable JSON artifact | ✅ |
| `report.ts` | deterministic duration, prerequisite, book, schema, and modality gap report | ✅ |
| `loader.ts` | reads the curriculum off disk | ⛔ (fs) |
| `cli.ts` | `validate` command + report | ⛔ (fs) |
| `report-cli.ts` | prints JSON or text for CI artifact capture | ⛔ (fs) |
| `book-cli.ts` | writes or checks generated chapters and their hash manifest | ⛔ (fs) |
| `modality-cli.ts` | writes or checks `core/lesson-modality.json` | ⛔ (fs) |
| `narration-cli.ts` | writes or checks the narration export and its hash manifest | ⛔ (fs) |
| `track-progress.ts` | registry/curriculum/book facts → top-level progress rows | ✅ |
| `track-progress-cli.ts` | rewrites or checks the marked README track table | ⛔ (fs) |

Only the modules marked `fs` touch the filesystem (declared in
`required_capabilities.json`); everything the app relies on is pure and unit-tested
against inline fixtures.

## Validation rules (the CI gate)

`validate()` returns a list of issues. **Errors** fail the build; **warnings** and
**info** are reported but tolerated (some fields — `romanization`, `etymology_hook`
— and the `data/scripts/*.json` character data are still being authored track by
track). It checks: every content lesson's `concept_tag` resolves (canonical or
namespaced), one realization per (concept, language), required fields present,
field shapes, script-glyph coverage (where script data exists), and core-concept
coverage (enforced only for tracks that declare `parity: complete`). The
integration test runs it against the real curriculum, so drift breaks CI.

For a lesson declaring `schema_version: 2`, `validateCurriculum()` additionally
enforces its canonical spine node, unique per-language sequence, 1–299 second
declared and computed duration, stable typed body sections, explicit
skill/mode/strand/register/variety metadata, same-language prerequisites, and
transitive knowledge closure. Each typed block must also author an
`hl-knowledge` directive. Block introductions must exactly account for the
lesson's introduced atoms; production and recall assessments must be declared by
the lesson and available from transitive prerequisites or an earlier block.
Blocks may also author compact JSON `hl-activity` directives immediately after
their knowledge boundary. Each compiled activity must use a stable lesson-prefixed
id, assess a non-empty subset of that block's atoms, provide an unambiguous
canonical answer plus explicit variants, include correct and incorrect feedback,
and declare a 1–299 second response budget. The compiler resolves those variants
without reading learner-facing Markdown.
Schema-v1 tracks remain readable during migration.

When `curricula` are supplied, the same validator also requires exactly one
`curriculum.json` per registered language. Every shared node must have an
explicit segment/omission/relocation ledger; every canonical realization and
schema-v2 lesson must be mapped; mapped prerequisite closure must be complete
and topologically earlier; and every non-shared support lesson in the path must
belong to exactly one typed extension. Repeated visits to the same shared node
remain legal through distinct ordered path segments.

## Scope note

Script/character-breakdown data (`data/scripts/*.json`, the "learn to write, piece
by piece" source) is authored incrementally, one script at a time, starting with
Telugu/Kannada/Malayalam/Gurmukhi. This package reads whatever is present and
degrades gracefully (coverage checks become warnings, then errors once a script
file declares itself complete).
