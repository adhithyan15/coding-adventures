# HL38 — Retired gentle-ramp snapshots

**Status:** specification, 2026-09-20

**Supersedes:** HL33. Extends HL17 and HL24. Tracks #15671, parent #13193,
and curriculum program #12206.

## 1. Outcome

The gentle-ramp report is a deterministic projection of canonical lessons,
curricula, chapter policy, script inventories, and chapter ledgers. The former
`core/gentle-ramp-snapshots/` tree stored 851 exact-current-value copies: 23
languages times one metadata file, 26 metric files, and ten finding files.
Ordinary chapter work therefore rewrote language-wide generated facts without
adding an independent promise.

The tree is retired. Reports and the historical
`loadGentleRampSnapshotTracks()` API derive the same `TrackGentleRamp` values in
memory. `check:gentle-snapshots` remains as a read-only compatibility command:
it rejects resurrection of the retired path, derives all registered tracks, and
checks exact registry closure. There is no write command or filesystem grant.

## 2. Authority classification

Every former metric owner is classified below. “Hard” means the measurement
feeds an independently enforced zero/integrity gate. “Policy-backed report”
means the threshold is authoritative in `core/chapter-policy.json`, while the
current corpus count is derived debt rather than policy. “Report” means the
value is useful for prioritization but is not an independently authored promise.

| Former metric owner | Classification | Authority after HL38 |
|---|---|---|
| `atomMeasurableLessons` | report | parsed lesson declarations |
| `atomMeasurementBlindLessons` | report | parsed lesson declarations; blindness remains named debt |
| `durationViolations` | hard | five-minute policy plus the corpus zero-violation integration gate |
| `atomLessonSpikes` | policy-backed report | `maxNewAtomsPerLesson` |
| `atomChapterSpikes` | policy-backed report | `maxNewAtomsPerChapter` |
| `glyphLessonSpikes` | policy-backed report | `maxNewGlyphsPerLesson` |
| `scriptSystemSpikes` | hard | `maxNewScriptSystemsPerLesson` plus language-owned zero gates |
| `scriptClosureViolations` | report | source-derived script-closure measurement and track-specific closure gates |
| `neverTaughtGlyphs` | report | source-derived script-closure measurement and track-specific closure gates |
| `orderDefects` | hard | curriculum order/dependency validation and language-owned zero gates |
| `lessonsWithoutSequence` | hard | language-owned explicit-order zero gates |
| `unknownPrerequisites` | hard | corpus integration zero gate |
| `forwardPrerequisites` | hard | language-owned dependency zero gates |
| `forwardReviews` | hard | language-owned dependency zero gates |
| `forwardReferences` | report | source-derived learner-language continuity queue |
| `atomsTaught` | report | source-derived continuity accounting |
| `atomsNeverRevisited` | report | source-derived continuity queue |
| `reinforcementWindowMisses` | report | source-derived R1–R4 continuity queue |
| `reinforcementMissesByWindow-R1` | report | R1 window definition and canonical lessons |
| `reinforcementMissesByWindow-R2` | report | R2 window definition and canonical lessons |
| `reinforcementMissesByWindow-R3` | report | R3 window definition and canonical lessons |
| `reinforcementMissesByWindow-R4` | report | R4 window definition and canonical lessons |
| `payoffSurprises` | report | source-derived chapter gate findings |
| `writingPracticeLessons` | report | canonical lesson modality; writing-stage readiness is gated separately |
| `firstWritingPracticeAt` | report | canonical reading order and lesson modality |
| `lessonsBeforeWritingPractice` | report | projection of `firstWritingPracticeAt` and lesson count |

The ten former finding owners add no authority of their own. `duration` and
`order-integrity` project hard measurements; `atom-step` and `glyph-step`
project authored budgets. `forward-language`, `script-closure`, `writing-ramp`,
`reinforcement`, `payoff-surprise`, and `measurement-blind` are deterministic
learner-first reporting. Their counts, units, details, ordering, and `next`
selection continue to be tested as report behavior rather than stored as data.

## 3. Gates retained

Retiring generated values does not delete their underlying controls:

- the five-minute ceiling and unknown-prerequisite corpus gates remain exact zero;
- chapter policy still supplies required atom, chapter, glyph, and script-system budgets;
- language-owned corpus tests keep complete order, prerequisites, reviews,
  measurement arithmetic, and the one-writing-system step fail closed;
- script closure, reinforcement, chapter payoff, writing-stage, level-readiness,
  and completion-plan measurements still derive from canonical sources; and
- `report:gentle-ramp` retains the deterministic full queue and JSON output.

Exact current positive debt is not a ceiling. Regenerating a file after a count
rises never prevented regression; it only recorded it. Independent promises stay
in policy or focused tests, while current debt stays visible in the report.

## 4. Filesystem retirement

Any direct child of `core/` whose name case-folds to
`gentle-ramp-snapshots` is an error, regardless of whether it is a file,
directory, or symbolic link. The check runs before public report loading and in
CI. The package exposes no generator and requests no snapshot write capability,
so a future aggregate cannot return as an accidental compatibility path.
