# HL49 — Complete Sanskrit assessment contract

**Status:** specification, 2026-10-13

**Extends:** HL15's completion plan and HL18's assessment-artifact gate. Tracks
#17474, the Sanskrit program #13415, and the Indian-language program #13295.

## 1. Activation boundary

Sanskrit withheld `assessment.json` while a machine-readable contract would
have pointed at files that did not exist. That boundary is now cleared. Every
rung from pre-A1 through C2 owns a four-skill task-shape inventory, two timed
source-mock forms, two answer keys, and a rung-specific rubric.

The contract may therefore become discoverable now. It records the complete
source-artifact envelope; it does not convert those source files into evidence
that the book or learner is ready.

## 2. Seven level-owned destinations

The contract declares exactly the policy-ordered rungs `pre-A1`, `A1`, `A2`,
`B1`, `B2`, `C1`, and `C2`. Each rung:

- names `Coding Adventures Sanskrit <Level> Assessment — project-defined
  equivalent` and cites the matching heading in `assessment-spec.md`;
- binds reading, listening, writing, and speaking to the matching section in
  `task-shapes/<level>.json`;
- requires an independent `0.6` pass threshold for every skill;
- names exactly two timed mocks, their level-owned answer keys, and their
  level-owned shared rubric; and
- accumulates every writing stage required by the repository assessment
  policy at that rung.

The contract owns no second task inventory, path registry, level ordering, or
pass policy. Those authorities remain the level-owned artifacts and the shared
assessment policy.

## 3. Fail-closed evidence

A Sanskrit-owned regression must parse the real contract through the shared
loader and prove all seven rungs are discoverable. For every rung it compares
the target with the real task-shape inventory, checks exact skill bindings and
thresholds, checks both timed mock identities and paths, and proves every
promised task inventory, rubric, and answer key exists through the shared
artifact audit.

The regression also proves that no mock declares a human-validation path.
Human-validation debt remains visible through the shared status projection as
two missing evidence records per rung rather than being hidden or invented.

## 4. Evidence boundaries

Activating the contract claims only that the checked-in source artifacts named
by it exist and satisfy their structural contracts. It must not claim:

- qualified recordings;
- independent human review or double-scored productive samples;
- standard setting or validated cut scores;
- book-only validation or learner proof;
- an external qualification; or
- mock, exam, or curriculum readiness.

`assessment-spec.md` remains the human-readable authority for these boundaries
and must describe `assessment.json` as an active source-artifact contract while
keeping all of the evidence above explicitly outstanding.

## 5. Acceptance

Completion requires:

1. the shared contract loader and assessment-artifact gate accept Sanskrit at
   every rung;
2. the completion planner no longer emits `assessment-contract/sanskrit`;
3. Sanskrit-owned tests prove exact paths, policy order, thresholds, writing
   stages, task-shape agreement, artifact presence, and absent human evidence;
4. existing Sanskrit task-shape and mock suites remain green; and
5. an independent security/data-integrity review reports no unresolved issue
   before every push.
