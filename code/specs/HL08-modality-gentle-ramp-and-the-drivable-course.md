# HL08 — Modality, the Gentle Ramp, and the Drivable Course

## Status and purpose

This spec makes two implicit properties of the curriculum explicit and checkable:

1. **Which channel a lesson needs** — can it be learned by ear alone, does it need
   eyes, does it need a pen? — marked with signs in the book and exported so a
   voice assistant can teach the hands-free parts aloud.
2. **How steep the ramp is** — how much new material a single lesson may introduce,
   enforced as a budget rather than left to authorial judgement.

It extends [HL00](./HL00-human-language-curriculum-framework.md), whose lessons are
already written in audio-script style with `[PAUSE Ns]`, `[REPEAT x2]` and
`[YOU SAY: …]` cues, and implements the **audio-script output** that
[HL04](./HL04-shared-spine-and-content-pipeline.md)'s one-source pipeline diagram
names but nothing has ever built. Chapter-level signs attach to the capability object
defined in [HL05](./HL05-chapter-capability-and-step-by-step-shape.md).

The design requirements are:

1. A learner in a car can be taught, aloud, everything that does not require eyes.
2. A lesson that needs eyes or a pen says so, visibly, before the learner starts it.
3. The ramp is gentle by measurement, not by assertion.
4. Length is not a cost. A longer book made of smaller steps is the preferred outcome.

## The gap this closes

### Modality is real but undeclared

`skills: [listening, speaking, reading, writing]` already exists on every schema-v2
lesson — but it records what a lesson **develops**, not what it **requires**. 501 of
the 531 schema-v2 lessons declare `[listening, speaking, reading]`, and a reader can
learn *hola* perfectly well by ear despite the `reading` entry. Modality therefore
cannot be derived from `skills`, and treating the two as the same thing would
mislabel almost the entire corpus.

Deriving instead from lesson type and block structure, measured over all 1,096 lessons:

| Requirement | Lessons |
|---|---|
| `type: writing` — needs a pen | 51 |
| Carries a `script` block — needs eyes | 7 |
| Neither | 1,038 |

That 1,038 is not the drivable count, because a lesson can be sight-dependent without
a script block:

| Among those 1,038 | Lessons |
|---|---|
| Contain a Markdown table | 322 |
| Contain a sight cue (*"see the"*, *"look at"*, *"the chart"*, *"column"*) | 56 |
| **Genuinely free of both** | **695** |

So roughly **63% of the corpus is drivable exactly as authored**, and the single
largest obstacle to the rest is the table — not the script. That is a tractable
problem: a two-column word→gloss table reads aloud fine, while a five-column paradigm
does not.

### The ramp is already gentle, and undefended

Knowledge atoms introduced per schema-v2 lesson:

| Statistic | Value |
|---|---|
| Mean | 2.31 |
| Median | 2 |
| p90 | 3 |
| Max | **7** (`ES-C31-numeros-11-20`) |
| Lessons introducing more than 3 | 52 |
| Lessons introducing more than 5 | 5 |

The curriculum is already gentle in aggregate. What it lacks is a floor: nothing stops
the next lesson from introducing nine atoms, and the current worst case teaches ten
numbers at once — precisely the "drilling ten greetings" antipattern HL00 was written
to reject.

## Modality

Three channels, each naming what the learner must have available:

| Value | Meaning | Sign |
|---|---|---|
| `voice` | Learnable by ear alone. A voice assistant can teach it while the learner drives. | 🚗 |
| `sight` | Needs eyes — letter shapes, figures, or a table that cannot be read aloud. | 👁 |
| `pen` | Needs a hand — handwriting formation and practice. | ✍ |

Modality is **monotonic**: `pen` implies `sight`. A chapter's modality is the union of
its lessons'.

### Derived by default, overridable with a reason

Hand-annotating 1,096 lessons invites drift, so modality is computed:

1. `type: writing` → `pen`;
2. otherwise a `script` block, a sight cue, or a table wider than the configured
   linearisable width → `sight`;
3. otherwise → `voice`.

An author may override with an explicit `modality:` in frontmatter, but an override
that contradicts the derivation requires a `modality_reason:`. The validator reports
unexplained overrides. This keeps the common case free and the exceptional case honest.

### The drivable prefix

A chapter reports how many of its lessons, **in authored order**, are `voice` before
the first one that is not. This is the number that matters to a commuting learner:
*"you can do the first six of this chapter's nine lessons in the car."*

The book prints it at the chapter opening beside the capability from HL05. It is
derived, never authored.

## The narration export

A fourth output view beside the book, the app, and the exercise bank — the one HL04's
pipeline diagram already promises.

For every lesson and chapter, `narration-cli` emits from the canonical AST:

- **Plain text** — a continuous script an AI voice assistant or TTS engine can read.
- **Structured JSON** — blocks, cues, and prompts with their types preserved, so a
  voice agent can pause where the lesson says pause, wait for a spoken answer where
  the lesson says `[YOU SAY: …]`, and score it against the compiled activity contract
  from HL-V03 rather than against prose.

Rules:

- The existing `[PAUSE Ns]`, `[REPEAT x2]` and `[YOU SAY: …]` cues are preserved as
  structured directives, not flattened into prose.
- **Tables are linearised into speech**, not dropped. A two-column table becomes a
  sequence of *"X means Y"* utterances. A table that cannot be linearised within the
  configured width marks its lesson `sight` — the export never silently omits content
  the learner would then not know they had missed.
- `sight` and `pen` lessons still export, prefixed with a spoken notice naming what
  the learner will need and what they can safely skip until they stop driving.
- Target-language text carries its `romanization` alongside, so a voice engine reading
  a Latin-script transcription is never guessing at the script.
- The export is hash-gated against the lesson AST exactly like generated `.tex`, so
  narration cannot drift from the book.

The export is a **script for a voice agent, not a recording**. Producing audio,
selecting voices, and provenance-labelling recordings remain out of scope, as in HL04.

## The gentle-ramp budget

A new configured limit in `core/chapter-policy.json`:

- `maxNewAtomsPerLesson` — default **3**, at the corpus's existing p90.
- `maxNewAtomsPerChapter` — a chapter-scoped ceiling, so gentleness cannot be gamed by
  splitting one steep lesson into two steep ones.

Exceeding the budget is a violation to be **split, not waived**. This is the same move
the HL-D01 series already made for the five-minute rule: the fix for a lesson that
introduces seven atoms is more prerequisite-ordered lessons, each teaching less.

52 lessons currently exceed a budget of 3, and 5 exceed 5. That is the burn-down list,
recorded as debt rather than treated as a new regression.

### Length is explicitly not a cost

Splitting for gentleness makes every book longer, and that is the intended direction.
A book of thousands of pages made of two-minute steps is a better outcome than a
compact book that loses the reader in chapter three. No gate in this project may
penalise page count, lesson count, or chapter count.

## Validation gates

Added to `validateCurriculum()`, multi-pass, collecting every violation before
reporting.

| Code | Rule |
|---|---|
| `modality-unexplained-override` | an authored `modality` contradicting the derivation has no `modality_reason` |
| `modality-unknown-value` | `modality` is not one of `voice`/`sight`/`pen` |
| `narration-block-unrenderable` | a block cannot be linearised into speech and the lesson is not marked `sight` |
| `ramp-budget-exceeded-lesson` | a lesson introduces more than `maxNewAtomsPerLesson` |
| `ramp-budget-exceeded-chapter` | a chapter introduces more than `maxNewAtomsPerChapter` |
| `narration-drift` | the committed narration export does not match the lesson AST |

All land **report-first**, per the HL-V01 precedent, and flip to errors per track as
each track's debt clears.

The gap report gains a modality section: per track, the count of `voice`/`sight`/`pen`
lessons, each chapter's drivable prefix, and the corpus-wide drivable percentage — so
"how much of this can I do in the car?" is a measured number, published every build.

## Migration order

1. **Derivation and report** — modality computed for all 1,096 lessons; drivable
   prefixes and ramp debt published. No gates.
2. **Signs in the book** — chapter openings print modality and drivable prefix beside
   the HL05 capability; lessons carry an inline marker.
3. **Narration export** — `narration-cli` with `--write`/`--check`, table
   linearisation, and the hash gate.
4. **Table remediation** — the 322 table-bearing lessons reviewed; each table either
   linearised or its lesson honestly marked `sight`.
5. **Ramp burn-down** — the 52 over-budget lessons split into prerequisite-ordered
   micro-lessons, longest first.

## Acceptance criteria

Complete when every lesson has a derived or explained modality; every chapter reports
a drivable prefix; the book prints modality signs at every chapter opening; the
narration export round-trips every lesson and is hash-gated against the AST; every
table is either linearised for speech or its lesson is marked `sight`; no lesson
exceeds the ramp budget; and the published gap report states what percentage of each
track can be learned entirely by ear.
