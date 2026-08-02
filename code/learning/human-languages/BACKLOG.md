# Human Languages Backlog

This is the ordered delivery backlog for the shared-spine curriculum, books,
and Language Ladder. Reprioritize it after every merged work item. Add newly
discovered work here before starting it so the repository, rather than an agent
session, remains the source of truth.

Last prioritized: 2026-08-02. Current baseline after PR #9472: 20 registered
tracks, 973 Markdown lessons, and 18 downloadable LaTeX books. The Urdu item
below adds the nineteenth book without adding a parallel publication workflow.

## Priority rules

1. Close a learner-visible broken promise before adding breadth.
2. Prefer work that makes later corpus growth measurable or generated.
3. Finish a small vertical slice before starting the same migration everywhere.
4. Keep the application, book, and canonical lesson content aligned.

## P0 — current publication and validation gaps

| ID | Status | Work item | Completion signal |
|---|---|---|---|
| HL-B01 | Complete (#9472) | Publish the five authored Persian lessons as a two-chapter LaTeX starter book. | XeLaTeX builds the book; CI discovers an 18th PDF; chapters map to lessons 1 and 2. |
| HL-B02 | Complete in the Urdu starter-book PR | Publish the five authored Urdu lessons as a two-chapter starter book. | XeLaTeX builds with correct RTL shaping; Urdu appears in the public catalog. |
| HL-B03 | Next | Publish Russian's two authored chapters as a starter book. | The existing Cyrillic lessons and roadmap produce a downloadable PDF. |
| HL-V01 | Queued | Add a machine-readable curriculum gap report and computed duration budget. | CI reports lessons at or above 300 seconds, missing prerequisites, book coverage, and track-schema status. |

## P1 — one-source migration

| ID | Work item | Why it follows P0 |
|---|---|---|
| HL-S01 | Migrate Spanish Chapters 1–3 to schema version 2 with typed body blocks and knowledge closure. | Spanish is the specified vertical slice and proves the strict contract on mature content. |
| HL-G01 | Generate a Spanish LaTeX chapter from the canonical lesson AST and compare source hashes with the app. | Removes the first handwritten book copy only after the AST contract is proven. |
| HL-D01 | Split or rewrite every lesson whose computed duration is at least 300 seconds. | The audit must exist first so the debt remains measurable throughout migration. |
| HL-M01 | Add per-track spine realization maps and language-specific extension nodes. | Enables safe cross-language scheduling beyond the current concept join. |
| HL-T01 | Complete session maps and pronunciation references for Persian and Urdu. | The starter-book work supplies both roadmaps and changelogs; these remaining pieces complete the standard track shape. |
| HL-U01 | Vendor and verify an appropriately licensed static Nastaliq font for normal Urdu presentation. | Naskh remains an explicit accessibility fallback, not the intended printed style. |

## P2 — corpus growth

- Extend Persian and Urdu through the first three shared-spine clusters.
- Expand every track toward B1 using the gap report to choose the next missing
  can-do, skill, mode, register, or realization.
- Add controlled dialogues and micro-stories whose tokens are validated against
  prior knowledge.
- Add provenance-labelled listening and dictation activities from the same
  canonical lesson blocks.

## Completed foundations

- HL04 defines the 45-concept shared spine and migration contract.
- The 20-track registry, Persian/Urdu pilots, full Markdown bodies, registry-driven
  language selection, RTL app rendering, and fail-closed prerequisites are merged.
- One CI job now installs TeX once, compiles every book, uploads one publication
  bundle, and publishes the catalog after changes reach `main`.
