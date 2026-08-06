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
| `report.ts` | deterministic duration, prerequisite, book, and schema gap report | ✅ |
| `loader.ts` | reads the curriculum off disk | ⛔ (fs) |
| `cli.ts` | `validate` command + report | ⛔ (fs) |
| `report-cli.ts` | prints JSON or text for CI artifact capture | ⛔ (fs) |
| `book-cli.ts` | writes or checks generated chapters and their hash manifest | ⛔ (fs) |

Only `loader.ts`, `cli.ts`, `report-cli.ts`, and `book-cli.ts` touch the filesystem (declared in
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
