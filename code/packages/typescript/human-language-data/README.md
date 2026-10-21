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

The Human Languages progress table is derived too. `npm run report:progress`
prints the current registry-ordered table on demand without rewriting tracked
documentation; CI runs `npm run check:progress` so the 23 retired progress-card
rollups cannot return.

Full assessment mocks name a learner-facing `paper` separately from their
rubric and answer key. `npm run check:assessment-artifacts` verifies every
declared file, rejects path and symlink escapes, and keeps legacy paper
omissions visible in exact per-track pins under
`core/assessment-paper-omission-ceiling/`. These pins cannot grow; an omitted
paper is unfinished evidence, not exam readiness. See the
[full-mock paper contract](docs/feature-contracts/full-mock-paper-artifacts.md).
Each level mock can additionally declare a JSON `manifest` naming its four
skill timings and task-shape part IDs. A declared manifest must be a real file
in its track and must match one complete published form per skill, with
visible per-part spans in the learner paper, rubric, and answer key. The plan's
`mock-assessment/<language>/<level>` family counts mocks still missing a
distinct paper, rubric, key, or valid manifest; undeclared legacy manifests
remain visible debt rather than becoming a parse error.
For present full-mock evidence, the validator uses the repository's Python 3
handle-relative reader so a concurrent symlink or Windows junction swap cannot
redirect the manifest, paper, rubric, or key outside the audited track. Python 3
must be available as `python3` on Unix or `python` on Windows for this check;
an unavailable reader fails the gate rather than counting evidence as missing.
See the [structure-manifest contract](docs/feature-contracts/full-mock-structure-manifests.md)
for the JSON shape and migration rule.

```
lessons/*.md frontmatter    ─┐
<track>/curriculum.d/**/*   ─┤
<track>/curriculum-membership.d/* ─┼─►  Dataset + local realization paths
concepts/taxonomy.json      ─┤         + validate() / validateCurriculum()
data/scripts/*.{json,d/}    ─┘         + independent frontier planning
```

The core is the **concept**: a language-independent idea (`GREETING-HELLO`). Each
language's word for it is a **realization** (Spanish *hola*, Telugu *నమస్కారం*).
That join is what lets a study app say "here is *hello* in every language you're
learning."

### Authoring sharded ledgers

Every track's chapter and curriculum ledgers are conflict-resistant shards, as
are the shared spine, book-generation manifest, and Japanese, Perso-Arabic,
Tamil, and Urdu-Nastaliq script inventories. Edit the owning file under `X.d/`;
their monoliths do not exist. Book-generation chapters use stable
`<language>-<NNNN>.json` owners rather than per-language slices, and its
backmatter and script sets live in independently owned section directories.
The authored lesson-sound vocabulary uses `core/sound-tags.d/<language>.json`,
one self-binding `{ language, tags }` owner per registered track plus a stable
`_meta.json`; the former cross-language aggregate is not tracked.
The reading-reach ratchet uses
`core/reading-reach-floor.d/<language>--<level>.json`, one self-binding
`{ language, level, floor }` owner for every task-shape inventory. Zero owners
preserve the historical absent-entry-zero behavior; the reconstructed public
map contains only positive floors, and the former aggregate is forbidden.
Generated lesson modality uses the finer
`core/lesson-modality/<language>.d/{_meta.json,<lesson-id>.json}` boundary. The
metadata owner holds only stable derivation policy and each lesson file holds its
own row and findings; flat per-language aggregates are forbidden.
Curriculum path-to-spine membership is owned only by the new path shard's
`spine_node`. Do not edit an existing `curriculum.d/spine/*.json` owner when
adding a path segment: those files contain only stable `omits`/`relocates`
policy, and the public `segments` lists are derived in path order at load time.
Lesson membership is single-owned under
`<track>/curriculum-membership.d/<lesson-id>.json`. Each direct owner names one
path segment, its exact order there, and any attached extensions with their
exact order. Path and extension shards must not store `lessons` arrays. Adding
one lesson adds one membership owner rather than editing either shared array;
the loader and Language Ladder reconstruct the unchanged public arrays. See
[`HL40`](../../../specs/HL40-direct-curriculum-lesson-membership.md).
Browser consumers fold only their respective canonical shards behind bounded
build-time modules. Language Ladder does not import the modality owner corpus.

```bash
npm run check:shards
```

CI rebuilds every shard set in memory and rejects a resurrected aggregate. For
the 47 generic core-spine, chapter, and curriculum plans, it also compares the
logical owner set with independent graph evidence: curriculum maps prove the
core spine, generated narration chapter owners prove chapter capabilities, and
lesson/path/extension validation closes each curriculum section. A clean deletion,
unexpected owner, duplicate identity, filename/body mismatch, or case-fold
collision therefore fails even when the surviving shards still reconstruct.
Ordinal prefixes remain ordering coordinates, so an id-bearing owner inserted
at `0015` is accepted without renumbering its neighbours.

The Japanese, Perso-Arabic, Tamil, and Urdu-Nastaliq script inventories declare
`complete: false` because their learner repertoires are still growing. Their
current owner sets are nevertheless exact: one strict, ordinal-free declaration
under `data/script-owner-declarations/<script>/{letters,marks}/` independently
names every glyph or mark. `check:shards` compares both sets in both directions,
so a clean deletion, unexpected owner, duplicate identity, filename/body
mismatch, unsafe path, or case-fold collision fails without restoring a shared
script-sized manifest.

Static mutation evidence is independently sharded at the same stable boundary:
`data/script-owner-evidence/<script>/{letters,marks}/<identity>.json`. Each file
binds its glyph or mark to the SHA-256 of that inventory owner's canonical
bytes. `npm run check:script-owner-evidence` requires exact evidence/inventory
identity equality and matching per-owner bytes. A new glyph therefore changes
only its inventory owner, declaration, and evidence file; whole-script digests
and shared expected-count tables are no longer part of normal authoring.

Shared Markdown documents use `<document>.d/` and one strict, locale-free
filename grammar. `_meta.md` is the only metadata shard. Every section shard is
`RRRRR-SLUG-dddddddd.md`: a positive five-digit rank, one or more uppercase
ASCII letter/digit slug components separated by single hyphens, and the first
eight lowercase hexadecimal digits of SHA-256 over the raw heading line. Two
parallel fragments may deliberately share a rank; their slug and heading digest
make the full names distinct and code-unit sorting remains deterministic.
`npm run check:doc-shards` rejects every other new `*.md` basename before
rendering.

Indian-track roadmaps use the same boundary at level-2 planning sections.
Session maps use numbered Markdown table rows as their owners, including stable
letter-prefixed identities such as `S1`; table headings and surrounding prose
remain in `_meta.md`. The rendered `roadmap.md` and `session-map.md` files are
local views only, so adding or moving one chapter session changes its own owner
instead of a track-wide aggregate.

Eighty-five append-only history fragments predate that grammar. Their exact
document path, basename, and full-content SHA-256 are pinned in
`src/doc-shard-legacy.ts`; they render in place exactly once. The compatibility
set is closed: an unknown legacy-shaped addition or any edit to a grandfathered
body fails the check. Do not rename or reshard those files. Append a strict
rank/slug/digest fragment instead.

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

const { curricula, dataset, lessons, registry, scripts, soundTags, spine, taxonomy } = loadEverything();

// Typed activities compile directly from block metadata, never from prose.
const activities = compileLessonActivities(lessons[0].blocks);

// The cross-language join:
languagesForConcept(dataset, "GREETING-HELLO");
//  → [{ language: "spanish", headword: "hola", … }, { language: "telugu", … }, …]

// The consistency gate:
const issues = [
  ...validate({ taxonomy, lessons, scripts, soundTags }),
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

## Feature contracts

Feature-specific operating contracts live as regular direct owners in
[`docs/feature-contracts/`](docs/feature-contracts/). The directory listing is
the discovery mechanism; it has no hand-maintained index, manifest, or aggregate.

## Architecture — a pure core with a thin fs shell

| Module | Role | Pure? |
|---|---|---|
| `frontmatter.ts` | tiny zero-dep frontmatter reader with one nested-map level | ✅ |
| `parse.ts` | frontmatter + Markdown → typed lesson AST; realizations → `Dataset` | ✅ |
| `activity.ts` | typed block activities → normalized runtime answer contracts | ✅ |
| `hash.ts` | stable canonical lesson serialization and deterministic fingerprints | ✅ |
| `curriculum-membership.ts` | direct lesson owners → exact public path/extension arrays | ✅ |
| `curriculum-membership-shards.ts` | strict per-lesson curriculum owner fold and filesystem boundary | ⛔ (fs) |
| `sound-tags.ts` | pure closed-vocabulary shape and tag validation | ✅ |
| `sound-tag-shards.ts` | strict per-language owner fold and filesystem boundary | ⛔ (fs) |
| `delivery-cue.ts` | the one `[PAUSE …]` / `[REPEAT …]` / `[YOU …: …]` cue grammar shared by the book and the narration | ✅ |
| `book.ts` | typed lesson AST → LaTeX chapter | ✅ |
| `curriculum.ts` | spine, realization-map, prerequisite, schema-v2 duration/block/knowledge validation | ✅ |
| `plans.ts` | ordered local paths, extension placement, next lessons, and mixed ready frontiers | ✅ |
| `validate.ts` | the round-trip validator (errors fail CI; warnings tolerated) | ✅ |
| `queries.ts` | `allConcepts` / `conceptsByLanguage` / `languagesForConcept` / `coverageByLanguage` | ✅ |
| `modality.ts` | per-lesson channel (voice/sight/pen) and per-chapter drivable prefix | ✅ |
| `drivable-instructions.ts` | the steps a narrator would say plainly that need eyes or hands (`eyes-or-hands-step`) | ✅ |
| `narration-cues.ts` | what one cue sounds like (spoken or deferred) and where a paragraph's cues are | ✅ |
| `for-the-ear.ts` | a typed activity prompt and a heading, reworded for a listener ("Type …" → "Say …"; a label without its asking) | ✅ |
| `speech.ts` | Markdown → speakable words; Markdown tables → spoken utterances or a reasoned refusal | ✅ |
| `narration.ts` | typed lesson AST → narration segments and the continuous voice script | ✅ |
| `modality-manifest.ts` | derived modality rows, rollups, and unchanged filterable public manifest | ✅ |
| `modality-shards.ts` | strict direct-owner fold and filesystem boundary | ⛔ (fs) |
| `gentle-ramp-retirement.ts` | rejects resurrection of retired generated snapshots | ⛔ (fs) |
| `report.ts` | deterministic duration, prerequisite, book, schema, and modality gap report | ✅ |
| `lesson-budgets.ts` | explicit idiom, sense, and culture-claim budget measurement | ✅ |
| `loader.ts` | reads the curriculum off disk | ⛔ (fs) |
| `lesson-cache.ts` | test-suite-only, content-addressed cache of each track's parsed lessons | ⛔ (fs) |
| `cli.ts` | `validate` command + report | ⛔ (fs) |
| `report-cli.ts` | prints JSON or text for CI artifact capture | ⛔ (fs) |
| `book-cli.ts` | writes or checks generated chapters and their hash manifest | ⛔ (fs) |
| `modality-cli.ts` | writes or checks `core/lesson-modality/<language>.d/*.json` | ⛔ (fs) |
| `gentle-ramp-snapshot-cli.ts` | compatibility check: rejects retired snapshots and derives every track | ⛔ (fs) |
| `narration-cli.ts` | writes or checks the narration export and its hash manifest | ⛔ (fs) |
| `track-progress.ts` | registry/curriculum/book facts → progress rows and cards | ✅ |
| `track-progress-cli.ts` | prints the progress table or rejects tracked rollups | ⛔ (fs) |

Only the modules marked `fs` touch the filesystem (declared in
`required_capabilities.json`); everything the app relies on is pure and unit-tested
against inline fixtures.

### Running the tests: the parsed-lesson cache

About 200 of the suite's test files read the real corpus, and Vitest runs each
file in its own module graph, so each one used to parse all ~38,000 lessons
again (123 full-corpus parses and ~1,250 s of CPU per run). The suite now turns
on `src/lesson-cache.ts`:

- `tests/setup/warm-lesson-cache.ts` (Vitest `globalSetup`) parses the real
  corpus once, before any worker starts, and stores one entry per track under
  `node_modules/.cache/human-language-data/lessons/`.
- `tests/setup/lesson-cache.ts` (Vitest `setupFiles`) turns the cache on in
  every worker, for the real curriculum root only.

Each entry is named by a SHA-256 of that track's lesson **bytes**, its script,
the package's own source and the V8 version, so editing any lesson (or any
parser code) is a miss and can never be answered from an old parse. Entries
are read only from a private (0700) directory, only when they are regular 0600
files owned by the current user, never through a symlink, and only when their
embedded SHA-256 matches; anything else is a miss and the lessons are parsed
from disk. On Windows, which has no owner or mode bits, those two checks are
skipped and in exchange the cache is used only in that one fixed directory;
links and junctions, entry names and digests are still checked. (Anyone who
can write there can already replace code in `node_modules`.) The cache is off
in every CLI process — `generate:*` and `check:*` always parse from disk.
Deleting `node_modules/.cache` is always safe.

## Validation rules (the CI gate)

`validate()` returns a list of issues. **Errors** fail the build; **warnings** and
**info** are reported but tolerated (some fields — `romanization`, `etymology_hook`
— and the `data/scripts/*.json` character data are still being authored track by
track). It checks: every content lesson's `concept_tag` resolves (canonical or
namespaced), one realization per (concept, language), required fields present,
field shapes, script-glyph coverage (where script data exists), and core-concept
coverage (enforced only for tracks that declare `parity: complete`). The
integration test runs it against the real curriculum, so drift breaks CI.

The direct etymology-gap owner tests reject linked roots and children on every
platform. Windows uses a directory junction when native symlink privilege is
unavailable; Linux CI still exercises the real file-symlink case. The test
asserts that each fixture is actually a link before checking the loader's
refusal, so a missing link capability cannot silently turn this guard green.
Other package link-rejection fixtures likewise use junctions for directory
links on Windows. File-link checks are separate: they run when Windows grants
that capability and remain mandatory on Linux CI.

`tests/root-homonym-regressions.test.ts` pins the corrected real-corpus root
slugs for cross-track homonyms and checks the cousin join on representative
positive and negative pairs (nine/new, garland/dirt, black/art, free/book).
It reads only the named lessons, so the regression stays fast as the full
curriculum grows.

Exact script-inventory evidence is partitioned by ownership under
`tests/script-inventories/`. A stable glob in `tests/integration.test.ts`
discovers those evidence modules while loading the corpus once. Japanese, Urdu,
Tamil, and other script authors add or update only their inventory's evidence
file; the integration gate remains a cross-corpus entry point rather than a
shared edit surface.

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

## Completion-plan assessment readiness

The completion plan keeps a missing seven-rung `assessment.json` visible as
debt, but does not put it in the actionable head while its task shapes or timed
mock source papers, answer keys, and rubrics are absent. Its `blocked` output
names those prerequisites per track and level; the projection still counts the
contract. This is a conservative planning screen, not permission to publish a
contract: the assessment-contract and artifact gates remain authoritative, and
new contracts must never name missing evidence. See
[`HL15`](../../../specs/HL15-the-completion-plan.md).
The separate `mock-assessment` item is blocked on an absent task-shape
inventory; it cannot infer a complete timed paper from `timed: true` alone.

For a human review of Spanish mock question vocabulary, run
`npm run report:mock-stem-coverage -- --level A2` (or `pre-A1`/`A1`). The
report lists unaccounted and plausibly derivable stem/option forms; it is not
a pass/fail gate. Unknown levels are rejected before corpus reads. Valid
reports read Spanish lessons, curriculum membership, and the shared spine,
not every other language track. The taught-form conversion remains shared
with the Spanish mock audit, so narrowing the read does not narrow what the
report credits or hides.

## Scope note

Script/character-breakdown data (`data/scripts/*.json`, the "learn to write, piece
by piece" source) is authored incrementally, one script at a time, starting with
Telugu/Kannada/Malayalam/Gurmukhi. This package reads whatever is present and
degrades gracefully (coverage checks become warnings, then errors once a script
file declares itself complete).
