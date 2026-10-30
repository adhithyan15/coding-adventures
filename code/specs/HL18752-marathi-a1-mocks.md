# HL18752 — Marathi A1 full mock pair

Issue: [#18752](https://github.com/adhithyan15/coding-adventures/issues/18752)
Parent: [#13459](https://github.com/adhithyan15/coding-adventures/issues/13459)

## Problem

The Marathi assessment contract declares two timed A1 forms, and the published
task shape fixes eleven parts across reading, listening, writing, and speaking.
The instructional form and message runways are now present, but neither learner
paper, answer key, examiner script, rubric, nor fail-closed manifest exists.
The completion planner therefore reports both forms as missing.

## Evidence boundary

The Council of Europe CEFR Companion Volume supplies only generic A1 can-do
direction. The committed Marathi assessment specification, A1 task shape, and
taught curriculum are the controlling sources for task count, timing, scoring,
replay, aids, response length, and learner-facing language. No external Marathi
exam is mapped onto this project-defined paper.

Every Marathi form used in a stimulus, script, key, or accepted response must
be traceable to the committed curriculum. These structural artifacts do not
establish external certification, recording provenance, qualified recordings,
independent human validation, standard setting, learner readiness, or learner
attainment. Those obligations remain open on #13459.

## Required artifacts

The implementation adds, under `marathi/mocks/a1/`:

- two distinct learner papers;
- one separate answer key and one separate examiner script for each form;
- one shared analytic rubric; and
- one manifest per form.

Each manifest declares all eleven published task-shape part identifiers. Each
paper, key, and rubric contains one unique, non-empty evidence span for every
declared part. Listening and interlocutor scripts stay outside the learner
paper. A checked-in script is administration text, not evidence that a
recording, qualified speaker, or trained rater exists.

## Paper contract

Both forms preserve the task shape exactly:

- reading: 20 keyed items in 20 minutes, split 7 + 7 + 6, with section stimulus
  sets of 70–90, 90–120, and 90–140 words;
- listening: 17 keyed items in 18 minutes, split 6 + 6 + 5, with two plays at
  90–110 words per minute and stimulus windows of 60–90, 90–120, and 120–180
  seconds;
- writing: one 6–8-field practical form and one independent 30–40-word message
  on a continuous 20-minute clock, with prompt windows of 40–70 and 25–50
  words; and
- speaking: five interview questions, one prepared one-minute description, and
  one 60–120-second transactional role-play in ten minutes after five minutes
  of preparation, with the exact task-shape prompt windows.

Reading, listening, writing, and speaking remain separate 100-point papers.
Every paper must independently reach 60/100; an aggregate total cannot
compensate for a failed skill.

## Fail-closed validation

Level-owned tests first fail while the artifacts are absent. After
implementation they verify:

1. the assessment loader reports exactly two complete A1 mocks and no missing
   or blocked form;
2. all named papers, keys, scripts, manifests, and the shared rubric exist;
3. every manifest selects exactly the eleven A1 task-shape parts with exact
   section times;
4. every paper, key, and rubric owns one unique, non-empty span for every part;
5. objective item counts, distinct-form content, script separation, stimulus
   windows, response windows, replay limits, and independent skill thresholds
   remain pinned; and
6. readiness and evidence-boundary caveats remain explicit.

The clean-build completion planner must move Marathi A1 from 0/2 to 2/2 and
reduce the global missing-mock count by exactly two. Because mock work is not
projectable until every track has a contract, no change to the global numeric
projection is expected.
