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

```
lessons/*.md frontmatter  ─┐
curriculum.json × 20       ─┼─►  Dataset + local realization paths
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

### Modality and the drivable course (HL08)

[`HL08`](../../../specs/HL08-modality-gentle-ramp-and-the-drivable-course.md) asks a
concrete question: **how much of this can I learn in the car?** `modality.ts` answers
it for every lesson and every chapter.

```ts
import { summarizeModality, deriveLessonModality, loadEverything } from "@coding-adventures/human-language-data";

const { lessons } = loadEverything();
const modality = summarizeModality(lessons);
modality.drivablePercent;              // 84 — the share learnable by ear alone
modality.tracks[0].chapters[0];        // { drivablePrefix: 5, firstNonVoiceLesson: "…", … }
deriveLessonModality(lessons[0]).reasons; // ["wide-table"] — why it needs eyes
```

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
order, are `voice` before the first that is not — deliberately not "how many voice
lessons does it contain", because chapters are prerequisite-ordered and a voice
lesson sitting behind a sight one is not reachable in the car.

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

`core/book-generation.json` declares each generated chapter. The generator
orders schema-v2 lessons by `sequence`, writes the LaTeX chapter, and records a
stable FNV-1a fingerprint in `core/generated-book-hashes.json`. The fingerprint
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
| `report.ts` | deterministic duration, prerequisite, book, schema, and modality gap report | ✅ |
| `loader.ts` | reads the curriculum off disk | ⛔ (fs) |
| `cli.ts` | `validate` command + report | ⛔ (fs) |
| `report-cli.ts` | prints JSON or text for CI artifact capture | ⛔ (fs) |
| `book-cli.ts` | writes or checks generated chapters and their hash manifest | ⛔ (fs) |
| `narration-cli.ts` | writes or checks the narration export and its hash manifest | ⛔ (fs) |

Only `loader.ts`, `cli.ts`, `report-cli.ts`, `book-cli.ts`, and `narration-cli.ts` touch the filesystem (declared in
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
