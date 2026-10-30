# HL18733 — Marathi pre-A1 full mock pair

Issue: [#18733](https://github.com/adhithyan15/coding-adventures/issues/18733)

## Problem

The Marathi assessment contract declares two timed pre-A1 forms, and the
published task shape fixes twelve parts across reading, listening, writing,
and speaking.  None of the learner papers, keys, rubric, examiner scripts, or
fail-closed manifests exists.  The completion planner therefore reports both
forms as missing.

## Evidence boundary

The Council of Europe CEFR Companion Volume supplies only generic pre-A1
descriptors.  The Maharashtra Directorate of Languages is only evidence that
real Marathi assessment infrastructure exists; it does not publish this
project-defined pre-A1 paper.  The committed Marathi assessment specification,
task shape, and taught curriculum are the controlling sources for task count,
timing, scoring, replay, aids, variety, and learner-facing language.

Every Marathi form used in a prompt, script, key, or accepted response must be
traceable to the committed curriculum.  These artifacts must not claim
external certification, recording provenance, qualified recordings,
independent human validation, standard setting, learner readiness, or learner
attainment.

## Required artifacts

The implementation adds, under `marathi/mocks/pre-a1/`:

- two distinct learner papers;
- one separate answer key and one separate examiner script for each form;
- one shared analytic rubric; and
- one manifest per form.

Each manifest declares all twelve published task-shape part identifiers.  Each
of the paper, key, and rubric contains a unique visible evidence span for every
declared part.  Listening scripts stay outside the learner paper.  Script files
are administration text, not evidence that a recording or qualified speaker
exists.

## Paper contract

Both forms preserve the task shape exactly:

- reading: 14 keyed items in 10 minutes, split 6 + 4 + 4;
- listening: 14 keyed items in 10 minutes, split 6 + 4 + 4, with every item
  delivered twice at 70–90 words per minute and an eight-second answer pause;
- writing: eight items in 12 minutes, split two delayed-recall words, four
  dictation items, and two independently produced responses; and
- speaking: six prompts in eight minutes, split two greeting/identity prompts,
  three familiar questions, and one short request, with at most one slow
  repetition per prompt.

Reading, listening, writing, and speaking remain separate 100-point papers.
Every paper must independently reach 60/100; an aggregate total cannot
compensate for a failed skill.

## Fail-closed validation

Level-owned tests must first fail while the artifacts are absent.  After the
implementation they verify:

1. the assessment loader reports exactly two complete mocks and no missing or
   blocked form;
2. all named paper, key, rubric, examiner-script, and manifest files exist;
3. both learner papers contain the required timings, Marathi script, exact
   per-skill item counts, and distinct content;
4. every manifest matches the twelve task-shape parts; and
5. keys and the rubric preserve independent skill thresholds and
   Marathi-specific Devanagari evidence without exposing a learner answer
   script.

The clean-build completion planner must move Marathi pre-A1 from 0/2 to 2/2
and reduce the global missing-mock count by exactly two.  Because mock work is
not projectable until every track has a contract, no change to the global
projectable-item total is expected.
