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
modality.drivablePercent;              // 65 — the share a hands-free view can deliver
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

`maxLinearisableTableColumns` defaults to **0** — until the HL08 narration exporter
can turn a two-column table into "*X* means *Y*", no table is speakable, and
claiming otherwise would let a learner silently miss content. Raise it via
`summarizeModality(lessons, { maxLinearisableTableColumns: 2 })` when the lineariser
lands.

A **chapter's drivable prefix** is how many of its lessons, in authored `sequence`
order, have a `voice` core before the first that does not — deliberately not "how many
voice lessons does it contain", because chapters are prerequisite-ordered and a voice
lesson sitting behind a sight one is not reachable in the car.

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
| `modality-manifest.ts` | that derivation as an emittable, filterable JSON artifact | ✅ |
| `report.ts` | deterministic duration, prerequisite, book, schema, and modality gap report | ✅ |
| `loader.ts` | reads the curriculum off disk | ⛔ (fs) |
| `cli.ts` | `validate` command + report | ⛔ (fs) |
| `report-cli.ts` | prints JSON or text for CI artifact capture | ⛔ (fs) |
| `book-cli.ts` | writes or checks generated chapters and their hash manifest | ⛔ (fs) |
| `modality-cli.ts` | writes or checks `core/lesson-modality.json` | ⛔ (fs) |

Only `loader.ts`, `cli.ts`, `report-cli.ts`, `book-cli.ts`, and `modality-cli.ts` touch the filesystem (declared in
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
