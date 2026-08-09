# HL05 — Chapter Capability and the Step-by-Step Shape

## Status and purpose

This spec adds a **capability layer above the existing lessons**: every chapter
declares what the reader can do when they finish it, and proves that claim with a
payoff the reader can deploy immediately.

It extends [HL00](./HL00-human-language-curriculum-framework.md) and
[HL04](./HL04-shared-spine-and-content-pipeline.md). It does **not** revise the
lesson atom. All 1,096 existing lessons keep their shape, their one-word depth, and
their etymology. Nothing in this spec rewrites authored content.

The design requirements are:

1. Every chapter states one **can-do capability** in the reader's own terms.
2. Every chapter ends with a **payoff** — a dialogue or task the reader can use today.
3. A payoff recombines only material the reader has already been taught.
4. A payoff must exercise a real share of what its own chapter introduced.
5. Productive constructions get a first-class lesson type, so a chapter can teach
   *"here is how you build any of these"* rather than only *"here is one more word."*
6. Nothing gates the reader before value arrives.

## Evidence boundary

The requested shape is the one popularised by step-by-step self-study grammars, of
which *Complete Spanish Step by Step* is the reference the project was asked to match.
This spec adopts a **structural pattern** — chapter-sized deployable capability, an
immediately usable end-of-chapter payoff, no gating preamble — and nothing else. No
text, example set, exercise, sequence, or table is reproduced, paraphrased, or
adapted from that book or any other copyrighted course. Every lesson, dialogue,
pattern, and exercise in this corpus is original and grounded in the project's own
etymological method.

## The gap this closes

Measured against the corpus at the time of writing:

| Observation | Value |
|---|---|
| Chapters across all tracks | 379 |
| Chapters carrying a declared goal | **0** |
| Chapters carrying a validated payoff | **0** |
| Chapter-level objects of any kind | **0** — `chapter` is an integer on each lesson |
| Nearest existing approximation | 27 `consolidation` extension nodes; 6 of them are genuinely chapter-shaped, the rest are boilerplate |

The only chapter-scoped record in the repository today is a `targets[]` entry in
`core/book-generation.json` carrying `title`, `label` and `output`. That is a LaTeX
build manifest. It has no pedagogical content and is not a suitable home for one.

The consequence is structural, not cosmetic: because nothing in the data model knows
what a chapter is *for*, nothing can check that finishing one leaves the reader able
to do anything. A reader can complete thirty lessons, hold thirty words with accurate
etymologies, and still not own a usable exchange.

## The chapter capability object

One file per track, `<track>/chapters.json`, canonical and hand-authored:

```json
{
  "version": 1,
  "language": "spanish",
  "chapters": [
    {
      "chapter": 1,
      "title": "Hola and Buenos Días",
      "label": "ch:first-words",
      "canDo": "I can greet someone at any time of day and respond when greeted.",
      "spineNodes": ["SPINE-MEET-GREET"],
      "payoff": {
        "lesson": "ES-C01-practice",
        "kind": "dialogue",
        "summary": "A four-line doorway exchange, start to finish.",
        "assesses": ["ES-DIALOGUE-GREET"]
      },
      "figures": ["fig/es-ch01-greeting-map"]
    }
  ]
}
```

Field contracts:

- `chapter` — integer, unique per track, must match at least one lesson's frontmatter
  `chapter`.
- `title` — the chapter's printed name. **Becomes canonical here.**
  `core/book-generation.json` stops owning `title` and `label` and derives both from
  this file, removing the existing duplication.
- `canDo` — one sentence, first person, in the reader's terms. Same voice as a spine
  node's `canDo`. This is the chapter's promise.
- `spineNodes` — the shared spine nodes this chapter realises. May be empty for a
  chapter made entirely of language-specific extensions.
- `payoff.lesson` — the lesson id that delivers the payoff. Normally a `practice`,
  `practice-mix` or `pattern` lesson.
- `payoff.kind` — `dialogue` | `task` | `production`.
- `payoff.assesses` — the knowledge atoms the payoff exercises.
- `figures` — optional figure ids, defined by [HL06](./HL06-visual-system.md).

`chapters.json` is authored intent, unlike `curriculum.json`'s `omits` and
`relocates` ledgers, which are recomputed caches. A validator may not rewrite it.

Two further chapter properties — its **modality signs** and its **drivable prefix** —
are defined by [HL08](./HL08-modality-gentle-ramp-and-the-drivable-course.md) and are
*derived*, never authored here. They print at the chapter opening beside the `canDo`,
so a reader sees both what the chapter will let them do and whether they can do it in
the car.

## Self-sufficiency — the rule that makes a chapter worth finishing

HL04's closed-world rule stands unchanged: a lesson may require only atoms
established by a transitive prerequisite. This spec keeps that rule **strictly**, and
adds a chapter-scoped consequence.

A chapter is **self-sufficient** when its payoff can be performed using only atoms
introduced by this chapter or an earlier one in the same track. Because lesson-level
closure already guarantees each lesson's own requirements, self-sufficiency is not a
new burden on authors — it is the existing rule stated at chapter scope so it can be
reported per chapter.

A chapter is **representative** when its payoff assesses at least a threshold share of
the atoms that chapter introduces. This is the load-bearing rule. Without it a payoff
is satisfiable by exercising one word, and a chapter can claim a capability it never
delivered. The threshold is a configured constant, initially **0.5**, recorded in
`core/chapter-policy.json` so it can be raised as the corpus matures rather than
hard-coded at a call site. That same policy file carries HL08's gentle-ramp budgets.

### Why the ramp is slower than the trade book's, and what is deliberately not built

Step-by-step trade grammars routinely drop an untaught word into an early dialogue
and gloss it in the margin. Strict closure forbids that, so early chapters here ramp
slightly more slowly: a chapter must actually teach everything its payoff uses.

The escape hatch is a **presented tier** — a `presents.knowledge` list whose atoms may
appear in a lesson with an inline gloss, are never assessed, and never count as
taught. It would relax HL04 validation gate 3 for presented-only atoms while leaving
it fully strict for assessed ones.

**This tier is specified and deliberately not implemented.** The schema reserves the
`presents` key and validators must reject it as unknown until a later spec enables it.
Reserving it now means enabling it later is a flag flip, not a corpus migration.

## The `pattern` lesson type

`type: pattern` joins the non-lexical types exempt from `concept_tag` (alongside
`practice`, `review`, `writing`, `grammar`, `etymology`). It teaches a productive
construction:

- introduces exactly one `*-PATTERN-*` knowledge atom;
- declares `slots`, an ordered map from slot name to `fillers[]`, where every filler is
  a knowledge atom **already in the lesson's closure**. The repository's deliberately
  small YAML reader preserves authored map order and exposes the same data to books
  and apps as ordered `{ name, fillers[] }` objects;
- carries a `guided-production` block whose activity instantiates at least three
  distinct fillers;
- obeys every existing schema-v2 rule without exception — block directives, the
  five-minute budget, coverage metadata, block-level knowledge closure.

A `pattern` lesson is what lets a chapter generalise. `## Grammar Lens` stays exactly
where it is, inside a word lesson, for grammar that belongs to one word. The two are
complementary: the Lens explains why *this* word behaves this way, the pattern hands
the reader a productive template.

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

## Validation gates

Added to `validateCurriculum()` in `human-language-data/src/curriculum.ts`, following
that module's established multi-pass style — collect every violation, report once,
never throw on the first.

| Code | Rule |
|---|---|
| `chapter-missing-capability` | every chapter in a track's books has an entry with a non-empty `canDo` and a `payoff` |
| `chapter-unknown-payoff-lesson` | `payoff.lesson` resolves, and its `chapter` matches |
| `chapter-payoff-not-closed` | payoff atoms ⊆ atoms introduced by this or an earlier chapter in the track |
| `chapter-payoff-not-representative` | payoff assesses ≥ threshold share of the chapter's introduced atoms |
| `chapter-duplicate` | one entry per chapter number per track |
| `chapter-title-drift` | every `book-generation.json` target's title/label matches `chapters.json` |
| `pattern-slot-not-closed` | every declared slot filler is in the lesson's closure |
| `pattern-missing-production` | a `pattern` lesson has a `guided-production` block with ≥3 instantiations |
| `pattern-multiple-atoms` | a `pattern` lesson introduces exactly one `*-PATTERN-*` atom |

### Report first, fail later

Following the HL-V01 precedent, these gates land as **gap-report output over all 379
chapters**, not as errors. Existing debt is measured and made visible, never
retroactively treated as a regression. A track flips to hard errors only once its own
debt reaches zero, tracked per track exactly as the schema-v2 migration is.

The report gains a chapter section beside the existing duration and prerequisite
sections: chapters without a capability, payoffs that are not closed, payoffs below
the representativeness threshold, and per-track totals.

## Migration order

1. **Schema and loader** — types, `chapters.json` loader beside `loadLanguageCurricula`,
   `core/chapter-policy.json`. No gates yet.
2. **Report** — all nine checks as report-only; publish the first snapshot of all 379
   chapters.
3. **Title derivation** — `book-generation.json` stops owning `title`/`label`;
   `chapter-title-drift` proves the two agree through the transition.
4. **`pattern` type** — parser, validator rules, first patterns authored.
5. **Content fan-out** — capabilities and payoffs authored across all 20 tracks in
   parallel; gates flip to errors per track as debt clears.

## Acceptance criteria

The capability layer is complete when every one of the 379 chapters declares a
`canDo` and a payoff; every payoff is closed and representative under the configured
threshold; `book-generation.json` derives its titles rather than owning them; no
track reports chapter debt; the `pattern` type is in use in at least one chapter per
track; the book prints each chapter's capability at its opening; and Language Ladder
gates chapter completion on the payoff rather than on lesson count.
