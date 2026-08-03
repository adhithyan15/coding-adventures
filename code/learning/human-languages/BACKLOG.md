# Human Languages Backlog

This is the ordered delivery backlog for the shared-spine curriculum, books,
and Language Ladder. Reprioritize it after every merged work item. Add newly
discovered work here before starting it so the repository, rather than an agent
session, remains the source of truth.

Last prioritized: 2026-08-02. Current baseline after the Punjabi duration
tranche: 20 registered tracks, 983 Markdown lessons, and 20 downloadable LaTeX
books. HL-V01 makes the remaining migration debt reproducible in both JSON and
human-readable reports; HL-S01 proves the strict schema on the first 24 Spanish
lessons, and the HL-D01 tranches prove duration remediation without discarding
deep content.

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
| HL-D01A | Complete in the Russian duration PR | Remove all nine sub-five-minute violations from the complete Russian starter track. | The report measures zero Russian violations; every changed or added lesson is below 300 effective seconds. |
| HL-D01B | Complete in the Marathi duration PR | Remove all eight sub-five-minute violations from the Marathi track. | The report measures zero Marathi violations; the one genuinely long lesson is now two prerequisite-ordered micro-lessons. |
| HL-D01C | Complete in the Gujarati duration PR | Remove all nine sub-five-minute violations from the Gujarati track. | The report measures zero Gujarati violations; the one genuinely long lesson is now two prerequisite-ordered micro-lessons. |
| HL-D01D | Complete in the Punjabi duration PR | Remove all ten sub-five-minute violations from the Punjabi track. | The report measures zero Punjabi violations; the one genuinely long lesson is now two prerequisite-ordered micro-lessons. |
| HL-D01E | Next | Remove all ten sub-five-minute violations from the Sanskrit track. | Sanskrit is now the smallest remaining track-sized set; nine lessons need honest budgets and the 513-second numbers lesson needs a careful multi-part split. |
| HL-D01 | Queued | Split or rewrite every lesson whose computed duration is at least 300 seconds. | Deliver in measured track-sized tranches, beginning with HL-D01A, until the report reaches zero. |
| HL-S02 | Queued | Migrate Spanish Chapters 4–6 to schema v2 before generating their book chapters. | Chapters 1–3 prove generation; the next source slice must earn the same prerequisite and duration guarantees first. |
| HL-B04 | Queued | Publish Marathi Chapter 6 from its two canonical lessons rather than hand-copying another book chapter. | The duration audit exposed authored app content beyond the current five-chapter PDF; schema-v2 migration plus generation should close that drift safely. |
| HL-B05 | Queued | Remove Marathi's duplicate practice labels and Unicode bookmark warnings. | A forced build succeeds but reports four repeated `lesson:practice` labels, 32 Hyperref PDF-string warnings, and two underfull boxes; the clean-build signal is zero of each. |
| HL-B06 | Queued | Publish Gujarati Chapter 6 from its two canonical lessons rather than hand-copying another book chapter. | The duration audit exposed authored app content beyond the current five-chapter PDF; schema-v2 migration plus generation should close that drift safely. |
| HL-B07 | Queued | Remove Gujarati's missing punctuation glyphs and LaTeX layout/bookmark warnings. | A forced build succeeds but reports four missing punctuation glyphs, one overfull box, four underfull boxes, four duplicate practice labels, and 28 Hyperref warnings; the clean-build signal is zero of each. |
| HL-B08 | Queued | Publish Punjabi Chapter 6 from its two canonical lessons rather than hand-copying another book chapter. | The duration audit exposed authored app content beyond the current five-chapter PDF; schema-v2 migration plus generation should close that drift safely. |
| HL-B09 | Queued | Remove Punjabi's LaTeX layout, duplicate-label, and Unicode bookmark warnings. | A forced build succeeds with no missing glyphs but reports one overfull box, four underfull boxes, four duplicate practice labels, and 28 Hyperref warnings; the clean-build signal is zero of each. |
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

## Findings from HL-D01A

- Russian now has zero duration violations. The repository snapshot contains
  980 lessons and 472 violations overall, down from 481 before this tranche;
  unknown prerequisites remain at zero.
- Four lessons already computed below five minutes and only needed their
  declared estimates corrected. The five genuinely long lessons were shortened
  through de-duplication or split into four prerequisite-ordered support and
  practice lessons.
- The cross-language formality comparison, naming-as-action comparison, person
  shapes, and precise zero-copula explanation remain in the canonical corpus.
  The tightest changed lesson is `RU-C01-privet` at 293 computed seconds; every
  other changed or new lesson has a larger buffer.
- Marathi's eight violations are the smallest remaining track-sized set, ahead
  of Gujarati's nine and Punjabi's and Sanskrit's ten each. HL-D01B is therefore
  the next bounded duration tranche after this PR merges.

## Findings from HL-D01B

- Marathi now has zero duration violations. The repository snapshot contains
  981 lessons and 464 violations overall, down from 472 before this tranche;
  unknown prerequisites remain at zero.
- Seven lessons already computed between 126 and 171 seconds and only needed
  honest four-minute declared budgets. The one genuinely long lesson computed
  at 321 seconds.
- That counting lesson is now a 163-second core followed by a 240-second
  etymology lesson. The analogy and retention explanations remain complete and
  prerequisite-ordered in the canonical corpus consumed by Language Ladder.
- The audit also made a publication boundary explicit: Marathi Chapter 6 has
  canonical lessons but is not in the current five-chapter PDF. HL-B04 records
  the one-source migration and generation work instead of adding another manual
  copy.
- A forced build of the unchanged five-chapter book still succeeds with zero
  overfull boxes, but exposes four duplicate practice labels, 32 Unicode
  bookmark warnings, and two underfull boxes. HL-B05 records that pre-existing
  publication hygiene debt separately from the lesson remediation.
- Gujarati's nine violations are now the smallest remaining track-sized set,
  ahead of Punjabi's and Sanskrit's ten each. HL-D01C is therefore next after
  this PR merges.

## Findings from HL-D01C

- Gujarati now has zero duration violations. The repository snapshot contains
  982 lessons and 455 violations overall, down from 464 before this tranche;
  unknown prerequisites remain at zero.
- Eight lessons already computed between 110 and 184 seconds and only needed
  honest four-minute declared budgets. The one genuinely long lesson computed
  at 370 seconds.
- That counting lesson is now a 174-second core followed by a 253-second
  etymology lesson. The *dvé → be* inheritance, cross-Indic comparison, and
  restored *r* in *traṇ* remain complete and prerequisite-ordered in the
  canonical corpus consumed by Language Ladder.
- Gujarati Chapter 6 has canonical lessons but is not in the current
  five-chapter PDF. HL-B06 records its one-source migration and generation work
  instead of adding another manual copy.
- A forced build of the unchanged five-chapter book succeeds, but exposes four
  missing punctuation glyphs, one overfull box, four underfull boxes, four
  duplicate practice labels, and 28 Unicode bookmark warnings. HL-B07 records
  that pre-existing publication hygiene debt separately.
- Punjabi and Sanskrit tie for the smallest remaining set at ten violations,
  each with nine declaration-only lessons and one genuine split. Punjabi's long
  lesson computes at 405 seconds versus Sanskrit's 513, so HL-D01D takes Punjabi
  first as the safer bounded tranche.

## Findings from HL-D01D

- Punjabi now has zero duration violations. The repository snapshot contains
  983 lessons and 445 violations overall, down from 455 before this tranche;
  unknown prerequisites remain at zero.
- Nine lessons already computed between 106 and 172 seconds and only needed
  honest four-minute declared budgets. The one genuinely long lesson computed
  at 405 seconds.
- That lesson is now a 229-second counting-and-script core followed by a
  241-second etymology lesson. The Gurmukhi mark distinction, Chapter 5 callback,
  same-source *panjāh/pacās* evidence, and convergence explanation remain
  complete and prerequisite-ordered in the Language Ladder corpus.
- Punjabi Chapter 6 has canonical lessons but is not in the current
  five-chapter PDF. HL-B08 records its one-source migration and generation work
  rather than adding another manual copy.
- A forced build of the unchanged five-chapter book succeeds with no missing
  glyphs, but exposes one overfull box, four underfull boxes, four duplicate
  practice labels, and 28 Unicode bookmark warnings. HL-B09 records that
  pre-existing publication hygiene debt separately.
- Sanskrit's ten violations are now the smallest remaining track-sized set.
  Nine are declaration-only; its 513-second numbers lesson will require a more
  careful split than the three preceding tranches, so HL-D01E is next.

## Completed foundations

- HL04 defines the 45-concept shared spine and migration contract.
- The 20-track registry, Persian/Urdu pilots, full Markdown bodies, registry-driven
  language selection, RTL app rendering, and fail-closed prerequisites are merged.
- One CI job now installs TeX once, compiles every book, uploads one publication
  bundle, and publishes the catalog after changes reach `main`.
