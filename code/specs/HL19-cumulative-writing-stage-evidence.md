# HL19 — Cumulative writing-stage evidence

**Status:** specification, 2026-08-20

**Extends:** HL16 and HL17

**Durable work item:** [#12211](https://github.com/adhithyan15/coding-adventures/issues/12211)

## 1. The distinction this gate preserves

A lesson can contain a pencil icon, a copied word, or a text field without
proving that the learner has reached a writing capability. HL19 gives credit
only to an explicit assessed block on the cumulative writing ladder in HL16.

This is deliberately stricter than the gentle-ramp report's first-writing
measurement. That measurement asks whether writing practice begins early. This
one asks what the learner can now do, at which level, and whether every gentler
prerequisite was established first. Both are needed.

## 2. Evidence directive

A qualifying lesson block carries exactly one directive:

```markdown
## Writing — delayed recall
<!-- hl-knowledge: introduces=[]; assesses=[MW-SCRIPT-RAAM-01] -->
<!-- hl-writing-stage: delayed-copy -->

Look once, hide the model, write, reveal, and repair.
```

The directive must:

- appear immediately after the block's `hl-knowledge` metadata and before
  learner-facing copy;
- name one stage in `core/assessment-policy.json` exactly;
- occur in a lesson declaring the `writing` skill;
- sit in a block that assesses at least one knowledge atom;
- belong to a lesson with a derived curriculum level and finite `sequence`.

Authoring metadata is removed from learner-facing Markdown. A malformed or
misplaced directive is a schema-v2 validation error; it must never disappear as
though no evidence had been attempted.

## 3. The cumulative ladder

Stages are ordered by the assessment policy. A stage depends on every earlier
stage first required at the same or a lower level. This creates one intentional
fork:

```text
observe-trace
  -> guided-copy
  -> delayed-copy
  -> dictation-transcription
  -> controlled-composition
       -> timed-assessment-production (first required at A1)
       -> connected-composition      (first required at A2)
```

A1 timed production does not depend on the later A2 connected-composition
stage. At A2, both branches are required. Evidence authored out of order is
reported as `missing-stage-prerequisite` and receives no credit.

The level matrix is cumulative:

| level | writing stages required by the end of the level |
|---|---|
| pre-A1 | observe/trace, guided copy, delayed copy, dictation/transcription |
| A1 | pre-A1 stages, controlled composition, timed assessment production |
| A2–C2 | A1 stages plus connected composition |

Higher levels deepen length, genre, register, audience control, revision, and
timing through task-shape and assessment evidence. They do not erase the basic
orthographic capabilities below them.

## 4. Evidence is a proof point, not a terminal lesson

One valid block proves that a stage has been assessed at least once. It does not
prove durable mastery, task-shape coverage, mock readiness, or human validity.
Those remain separate gates. A book rewrite should use many tiny lessons and
retrieval opportunities before recording a stage proof.

Every instructional lesson remains at most five minutes. The writing-stage
directive never authorizes a long composition dump. Planning, opening a prompt,
choosing one detail, writing one sentence, revising one feature, and checking
one rubric dimension can each be their own lesson. Full timed papers remain
assessments, not instructional lessons.

## 5. Measurement and failure semantics

The data package measures every registered track, including tracks with no
evidence. For every `(track, level)` it publishes required, evidenced, and
missing stages. Evidence defects retain their lesson, block, stage, and reason:

- `unknown-stage`;
- `missing-writing-skill`;
- `empty-assessment`;
- `unmapped-level`;
- `unordered-evidence`;
- `missing-stage-prerequisite`.

Unmeasured is never clean. A missing track entry is treated as missing all
stages required at the queried level.

## 6. Level and backlog gates

The level-attainment gate gains a `writing-stage` criterion. A track cannot
attain a level while any cumulative stage required at that level is missing,
even when its spine, vocabulary, atom budget, and reinforcement criteria pass.

The completion plan emits a finite `writing-stage/<language>/<level>` item for
the first level each track is working on. Its outstanding count is the number of
missing required stages at that level. The projection counts all missing
`(track, level, stage)` pairs through the selected ceiling.

Assessment contracts and sourced task shapes stay ahead of book rewriting:
they define the target. Writing-stage work then comes before content inventory,
script closure, exam points, and vocabulary because every later content tranche
must be authored through the gentle productive ramp rather than retrofitted.

## 7. Initial baseline

At the 2026-08-20, 23-track baseline:

- 1 track has any explicit writing-stage evidence;
- Marwadi is the only track proving the four-stage pre-A1 ladder;
- 4 evidence blocks are valid and 0 are malformed;
- 1,007 cumulative `(track, level, stage)` pairs remain missing.

These figures are migration debt, not an assertion that other books contain no
writing. Existing writing receives no stage claim until an author reviews the
activity, decomposes any cliff, and adds explicit assessed evidence.

## 8. Acceptance criteria

HL19 is implemented when:

- directives parse strictly and stay out of learner copy;
- malformed attempts fail schema-v2 validation;
- cumulative prerequisite order is tested, including the A1/A2 fork;
- every registered track and all seven levels are measurable;
- missing writing stages block level attainment;
- the generated report and completion plan publish the debt;
- Marwadi supplies one real, gentle pre-A1 proof path;
- focused and corpus-wide tests pin the baseline without claiming exam readiness.
