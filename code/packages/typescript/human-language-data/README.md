# @coding-adventures/human-language-data

The machine-readable bridge from the **Human Languages** curriculum (spec
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md)) to the
cross-language dataset that downstream tools — the Engram practice deck
([`HL02`](../../../specs/HL02-companion-practice-app.md)) and anything later —
consume. It implements [`HL01`](../../../specs/HL01-concept-taxonomy-and-data-layer.md).

## What it does

The curriculum is a pile of per-language Markdown lessons. This package parses
their frontmatter, joins them through the canonical **concept taxonomy**
(`concepts/taxonomy.json`), and exposes the result as a queryable dataset —
plus a **validator** that keeps the lessons and the taxonomy from drifting apart.

```
lessons/*.md frontmatter  ─┐
concepts/taxonomy.json     ─┼─►  Dataset { concepts, byLanguage, languages }
data/scripts/*.json        ─┘         + validate() → Issue[]
```

The core is the **concept**: a language-independent idea (`GREETING-HELLO`). Each
language's word for it is a **realization** (Spanish *hola*, Telugu *నమస్కారం*).
That join is what lets a study app say "here is *hello* in every language you're
learning."

## Usage

```ts
import { loadEverything, languagesForConcept, validate } from "@coding-adventures/human-language-data";

const { dataset, taxonomy, lessons, scripts } = loadEverything();

// The cross-language join:
languagesForConcept(dataset, "GREETING-HELLO");
//  → [{ language: "spanish", headword: "hola", … }, { language: "telugu", … }, …]

// The consistency gate:
const issues = validate({ taxonomy, lessons, scripts });
```

### Architecture — a pure core with a thin fs shell

| Module | Role | Pure? |
|---|---|---|
| `frontmatter.ts` | tiny zero-dep YAML-frontmatter reader | ✅ |
| `parse.ts` | frontmatter → `Realization`; realizations → `Dataset` | ✅ |
| `validate.ts` | the round-trip validator (errors fail CI; warnings tolerated) | ✅ |
| `queries.ts` | `allConcepts` / `conceptsByLanguage` / `languagesForConcept` / `coverageByLanguage` | ✅ |
| `loader.ts` | reads the curriculum off disk | ⛔ (fs) |
| `cli.ts` | `validate` command + report | ⛔ (fs) |

Only `loader.ts`/`cli.ts` touch the filesystem (declared in
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

## Scope note

Script/character-breakdown data (`data/scripts/*.json`, the "learn to write, piece
by piece" source) is authored incrementally, one script at a time, starting with
Telugu/Kannada/Malayalam/Gurmukhi. This package reads whatever is present and
degrades gracefully (coverage checks become warnings, then errors once a script
file declares itself complete).
