# The two ramps (HL08)

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
2. otherwise `type: reading`, a `script` block, a sight cue, a table wider than the
   configured linearisable width, or an eyes-or-hands step → `sight`;
3. otherwise → `voice`.

An **eyes-or-hands step** (reason `eyes-or-hands-step`) is a sentence or a spoken cue,
in the preamble or a section a hands-free renderer keeps, that asks the learner to read
printed script, write, point, cover the page, handle cards or gesture — "[PAUSE 3s] Read
**नमस्ते**.", `[YOU RECALL: say *āmi*, then read **কেমন**]` — in a form the narration
says plainly. The detectors live in `drivable-instructions.ts`; a deferred cue
(`[YOU READ: **नमस्ते**]`, spoken "once you have stopped driving — read: नमस्ते") is not a
step, and is how a hands-on step stays inside a drivable lesson. The rule applies to
the core as well as the whole lesson, so `coreDrivable` can never be announced over one.

Two things the narration speaks are not part of that walk, and the renderer handles
them itself. A **scored activity** whose prompt needs eyes or hands ("Write son in
Chinese characters", `activityPromptNeedsEyesOrHands`) is deferred like a
`[YOU WRITE: …]` cue — `spoken: false`, "[once you have stopped driving — question:
…]". That check fails safe: beside the prose checks, any hand verb named as a whole
word ("From bachchā, write the taught word.") defers the question unless it is a
gloss, a noun or a third-person question (`handVerbsInPrompt`). A question whose
ANSWER is written is deferred too, whatever verb it uses: one asking for a written
form or a script ("… in two written lines", "Refuse in one word, in Devanagari"),
one offering a choice between unromanized glyphs ("… is na: ਨ or ਸ?"), one whose
every accepted answer is script ("Which taught character is … woman?" — 女), and one
that grades a `-SCRIPT-` atom with a script answer (`activityNeedsEyesOrHands`). A
separate gate scans the committed narration for those verbs, forms, choices and
answers with no detector at all. Every other prompt is said for the ear ("Type the Japanese for 'a stick'" →
"Say the Japanese for 'a stick'", `for-the-ear.ts`). In a lesson announced as drivable,
a **heading** that would ask for an eye or a hand is said as its label ("Guided
Practice — read, copy, recall" → "Guided Practice"), and the stop guard is spoken
before a set-aside section's title.

`type: reading` is the one place the word *reading* decides anything, and it is the
lesson **type**, not the skill: a reading lesson *is* printed text and the instruction
to read it, so it needs eyes even though every word of it could be spoken aloud
(reason code `reading-type`). Like `type: writing` it applies to the core too — the
text is the whole lesson, so there is nothing a hands-free view could set aside.

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
The book generator derives one `chapter-modalities.tex` projection per track with
font-independent car, eye, and pen signs, full printed-lesson counts, and the
core-based hands-free prefix. It also derives each track's glossary, answer key,
and subject index from the same canonical lesson and chapter owners. These 92
projections are compile inputs, not tracked curriculum: `check-book-compile.sh`
materializes them beneath a fresh temporary directory, exposes only the matching
track through `TEXINPUTS`, and removes the directory when the run exits. A clean
checkout needs the same local-dependency bootstrap as the all-books workflow:

```bash
(cd ../pixel-container && npm ci --omit=dev --ignore-scripts)
(cd ../paint-instructions && npm ci --omit=dev --ignore-scripts)
(cd ../paint-vm && npm ci --omit=dev --ignore-scripts)
(cd ../paint-vm-svg && npm ci --omit=dev --ignore-scripts)
npm ci --ignore-scripts
npm run build
npm run check:compile
```

A missing build fails clearly instead of reviving generated files in the source
tree. Generated and protected handwritten chapters both call the projection
immediately after their title and label, so a reader sees the requirement before
the first lesson and the book cannot drift from the app or narration model.

## One lesson, two modalities (HL-C41)

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
