# Gujarati

Gujarati is one track in the [Human Languages](../README.md) curriculum. It is
being built as a book that a learner can read from the first pre-A1 encounter
through a project-defined C2 examination. Every lesson is capped at five
minutes, new knowledge is introduced in small prerequisite-safe steps, and
writing grows from tracing to independent production rather than being left for
the end.

## What is distinctive here

- Gujarati is Indo-Aryan and uses the Gujarati script. The script is taught
  inside useful words and conversations, one unfamiliar form at a time.
- The script lacks the continuous top line used in Devanagari. A vendored Noto
  Sans Gujarati font keeps the generated book independent of host fonts.
- Gujarati retains masculine, feminine, and neuter agreement, has the copula
  *chhe*, and contains an important Perso-Arabic trade-language layer.
- Etymology is a memory tool, not a substitute for current meaning and usage.

## Assessment destination

The destination is the project-defined [Coding Adventures Gujarati
Assessment](assessment-spec.md), with separate passes in reading, listening,
writing, and speaking from pre-A1 through C2. The machine-readable
[assessment contract](assessment.json) requires the complete gentle writing
ladder and two timed mocks at every rung. It is a design target, not a claim of
external accreditation or present readiness.

The [pre-A1 task inventory](task-shapes/pre-a1.json) makes the first assessment
rung explicit. A1-C2 task inventories, mocks, rubrics, answer keys, calibration,
and book-only human pass evidence remain backlog.

## Current authored boundary

Canonical data currently contains **27 chapters and 165 lessons**. All 165
lessons are mapped and generated, and every lesson stays within the five-minute
cap. The pre-A1 writing-stage contract is complete, all 41 Gujarati forms shown
by the current book are explicitly taught, and the exact script inventory has
no never-taught glyphs.

That is a solid pre-A1 foundation, not a finished course. The exact pre-A1
headword inventory now contains 44 words. Two zero-new-atom checkpoints now
close the newest route performance's R1 window and the school/road meaning and
script R2 windows; the school lesson also returns city writing at R1. Current
measured continuity debt is pinned at 299 windows by the corpus tests rather
than treated as mastery. It still includes six atoms never revisited, two lesson
atom spikes, one chapter atom spike, two script-closure findings, and one
measurement-blind lesson. Vocabulary breadth, grammar, listening volume,
connected reading, free writing, speaking, every later level, and examination
materials still require substantial expansion.

| Chapters | Authored capability |
| --- | --- |
| 1-2 | Hear and say a greeting, learn its script gently, handle courtesy, and write a first answer from sound. |
| 3-7 | Learn every Gujarati form that the current book shows before conversation depends on it. |
| 8-10 | Exchange names, check wellbeing, and take leave. |
| 11-18 | Use first verbs and numbers, then build small domains around thought, reading, food, family, and the body. |
| 19-23 | Retrieve early script and conversation atoms at genuine third and fourth spacing windows. |
| 24-27 | Hear, say, read, and write ten concrete map words through separate four-skill checks, then retrieve the newest route atoms at measured R1/R2 intervals. |

The exact chapter titles, lesson ranges, and ordered lesson IDs are in the
[session map](session-map.md). The [roadmap](roadmap.md) separates this authored
boundary from the dependency-ordered work still needed for pre-A1 through C2.

## Sources of truth and validation

- `chapters.d/` and `curriculum.d/` are canonical chapter and curriculum data;
  `chapters.json` and `curriculum.json` are generated compatibility artifacts.
- `lessons/` contains the authored micro-lessons; `narration/` contains generated
  narration manifests.
- `book/` contains generated LaTeX chapters plus Gujarati-specific front matter.
- `progress/gujarati.md` is generated at the shared human-languages root and
  reports the current chapter and lesson counts.
- The Gujarati corpus test pins continuity, modality, writing stages, the gentle
  opening, and exact agreement between canonical order and this track's session
  map.

Run the shared generators and validators before publishing curriculum changes.
The generated PDF must also compile in strict mode and receive rendered-page
visual inspection; passing data tests alone does not prove readable Gujarati
shaping or layout.

## Book and font rules

The book uses the vendored `_fonts/NotoSansGujarati-Static.ttf` through
`fontspec`. Gujarati-script spans must stay inside `\gu{...}`; Latin
romanization and punctuation stay outside unless a generated span intentionally
contains them. Keep the font command local so page furniture and bookmarks do
not inherit it accidentally.
