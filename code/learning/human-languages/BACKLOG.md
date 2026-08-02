# Human Languages Backlog

This is the ordered delivery backlog for the shared-spine curriculum, books,
and Language Ladder. Reprioritize it after every merged work item. Add newly
discovered work here before starting it so the repository, rather than an agent
session, remains the source of truth.

Last prioritized: 2026-08-02. Current baseline after PR #9497: 20 registered
tracks, 976 Markdown lessons, and 20 downloadable LaTeX books. HL-V01 makes the
remaining migration debt reproducible in both JSON and human-readable reports;
HL-S01 proves the strict schema on the first 24 Spanish lessons.

## Priority rules

1. Close a learner-visible broken promise before adding breadth.
2. Prefer work that makes later corpus growth measurable or generated.
3. Finish a small vertical slice before starting the same migration everywhere.
4. Keep the application, book, and canonical lesson content aligned.

## P0 — current publication and validation gaps

| ID | Status | Work item | Completion signal |
|---|---|---|---|
| HL-B01 | Complete (#9472) | Publish the five authored Persian lessons as a two-chapter LaTeX starter book. | XeLaTeX builds the book; CI discovers an 18th PDF; chapters map to lessons 1 and 2. |
| HL-B02 | Complete (#9474) | Publish the five authored Urdu lessons as a two-chapter starter book. | XeLaTeX builds with correct RTL shaping; Urdu appears in the public catalog. |
| HL-B03 | Complete (#9478) | Publish Russian's two authored chapters as a starter book. | The existing Cyrillic lessons and roadmap produce a downloadable PDF. |
| HL-V01 | Complete (#9483) | Add a machine-readable curriculum gap report and computed duration budget. | CI reports lessons at or above 300 seconds, missing prerequisites, book coverage, and track-schema status. |

The Russian publication audit found eight of its thirteen Chapter 1--2
curriculum lessons currently declare five minutes or more, including one at six
minutes. This is concrete input to HL-V01 and HL-D01, not silently treated as
fixed by the book: the starter edition presents shorter dependency-ordered
micro-sections while the canonical duration split remains measurable debt.

The first deterministic HL-V01 snapshot measures 485 lessons at or above 300
effective seconds, zero unknown prerequisite ids, 42 later-chapter lessons with
no declared prerequisite, 257 lesson chapters without a matching book chapter,
and all 20 tracks still on the legacy lesson schema. The report is evidence for
the next migrations; it deliberately does not fail CI on already-recorded debt.

## P1 — one-source migration

| ID | Status | Work item | Why it follows P0 |
|---|---|---|---|
| HL-S01 | Complete (#9497) | Migrate Spanish Chapters 1–3 to schema version 2 with typed body blocks and knowledge closure. | The 24-lesson slice has unique order, transitive knowledge closure, typed blocks, and no effective-duration violation. |
| HL-G01 | Complete in the canonical-generation PR | Generate a Spanish LaTeX chapter from the canonical lesson AST and compare source hashes with the app. | Removes the first handwritten book copy now that the AST contract is executable. |
| HL-G02 | Complete in the Chapters 2–3 generation PR | Generate Spanish Chapters 2–3 from their canonical schema-v2 lesson AST. | Extends the proven one-source path across the rest of the migrated pilot before broad corpus work. |
| HL-D01A | Next | Remove all nine sub-five-minute violations from the complete Russian starter track. | A bounded first duration tranche covers both declaration-only and genuinely overlong lessons without skipping an existing language. |
| HL-D01 | Queued | Split or rewrite every lesson whose computed duration is at least 300 seconds. | Deliver in measured track-sized tranches, beginning with HL-D01A, until the report reaches zero. |
| HL-S02 | Queued | Migrate Spanish Chapters 4–6 to schema v2 before generating their book chapters. | Chapters 1–3 prove generation; the next source slice must earn the same prerequisite and duration guarantees first. |
| HL-M01 | Queued | Add per-track spine realization maps and language-specific extension nodes. | Enables safe cross-language scheduling beyond the current concept join. |
| HL-T01 | Queued | Complete session maps and pronunciation references for Persian and Urdu. | The starter-book work supplies both roadmaps and changelogs; these remaining pieces complete the standard track shape. |
| HL-U01 | Queued | Vendor and verify an appropriately licensed static Nastaliq font for normal Urdu presentation. | Naskh remains an explicit accessibility fallback, not the intended printed style. |

## P2 — corpus growth

- Extend Persian and Urdu through the first three shared-spine clusters.
- Expand every track toward B1 using the gap report to choose the next missing
  can-do, skill, mode, register, or realization.
- Add controlled dialogues and micro-stories whose tokens are validated against
  prior knowledge.
- Add provenance-labelled listening and dictation activities from the same
  canonical lesson blocks.

## Findings from HL-S01

- Spanish Chapters 1–3 contain 24 schema-v2 lessons after three overlong
  explanations were split into prerequisite-ordered support lessons for noun
  gender, the Latin *qu-* question family, and the origin of *usted*.
- The resulting snapshot has 976 lessons, 481 duration violations, and 40
  later-chapter prerequisite roots: four and two fewer respectively than the
  HL-V01 baseline, with the remaining debt still explicit.
- Every migrated lesson computes below 300 seconds; the tightest current budget
  is *buenos días* at 296 seconds, which should be watched during copy edits.
- Schema v2 now validates canonical spine mapping, unique local sequence,
  typed body blocks, explicit coverage metadata, same-language prerequisites,
  and transitive knowledge closure. Block-boundary prompt/answer knowledge
  declarations remain a later refinement; this slice does not claim them.

## Findings from HL-G01

- Spanish Chapter 1 is generated deterministically from seven canonical
  schema-v2 lessons in authored `sequence` order; the 18-book source is now 122
  rendered pages with no generated-chapter overfull boxes.
- The generated chapter and Language Ladder independently combine the same
  per-lesson FNV-1a fingerprints. The app exposes `book synced` only when its
  loaded Chapter 1 lesson AST matches the committed manifest.
- The unified book job now fails when generated TeX or the hash manifest is
  missing or stale. The fingerprint is a deterministic drift signal, not a
  cryptographic integrity claim.
- At the end of HL-G01, Chapter 1 was the first one-source slice and Chapters
  2–18 remained handwritten. That finding deliberately scoped HL-G02 to the
  already-schema-v2 Chapters 2–3 rather than skipping validation to generate
  later chapters.

## Findings from HL-G02

- All 24 schema-v2 Spanish lessons in Chapters 1–3 now generate their three
  LaTeX chapters and independently match Language Ladder's loaded AST. Chapter
  2 combines five lesson hashes; Chapter 3 combines twelve.
- The expanded canonical content produces a 138-page book. Rendered checks of
  both chapter openers, grammar and etymology boxes, nested emphasis, practice
  lists, and wrap-up recall found no generated-chapter overfull box or Hyperref
  warning.
- The renderer now handles nested bold-within-italic Markdown, wraps practice
  lists ragged-right, and keeps math arrows out of bookmark/running-header
  strings. Those fixes apply to every later generated chapter.
- The next learner-visible promise is the sub-five-minute cap. Russian is the
  smallest complete existing track with measurable debt: nine violations, of
  which five are computed at 312–405 seconds and four only need honest declared
  budgets below the cap. HL-D01A is therefore the next bounded tranche.

## Completed foundations

- HL04 defines the 45-concept shared spine and migration contract.
- The 20-track registry, Persian/Urdu pilots, full Markdown bodies, registry-driven
  language selection, RTL app rendering, and fail-closed prerequisites are merged.
- One CI job now installs TeX once, compiles every book, uploads one publication
  bundle, and publishes the catalog after changes reach `main`.
