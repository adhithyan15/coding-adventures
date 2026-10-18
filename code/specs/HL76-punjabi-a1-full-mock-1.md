# HL76 — Punjabi A1 full mock 1

## Status

Proposed for issue #17723 under the Punjabi mock umbrella #13463, the Punjabi
bridge #13384, and the Indian-language C2 programme #13295.

## Problem

Punjabi has a checked-in project-defined A1 task-shape inventory and names two
timed forms in `assessment.json`, but neither form binds a learner paper to a
confidential source master, a key, a rubric, and a structural manifest. The
completion planner therefore correctly reports two missing A1 mock tranches.

## Required artifact set

Make `a1-mock-1` the first structurally complete Punjabi A1 form by adding:

- one learner paper with twenty reading items, seventeen listening items, two
  writing tasks, and seven speaking prompts in the published 20/18/20/10-minute
  envelope, plus the published five-minute speaking preparation period;
- one confidential examiner and recording script holding every listening body
  and the live-interlocutor prompts outside the learner paper;
- one form-specific answer key;
- one shared A1 rubric that can score either declared form without a later
  form-specific rewrite; and
- one version-1 manifest binding exactly one visible span in the paper, rubric,
  and key to every published Punjabi A1 task-part ID.

The reading sources must stay inside their published aggregate word bands. The
three mastered listening parts must stay inside their published duration bands,
be delivered twice at 90–110 words per minute, and remain confidential. The
practical form requests six to eight independent Gurmukhi entries. The named-
reader message requests 30–40 independently written Punjabi words and supplies
no copyable answer model. Speaking administration must enforce the published
10–45/60–120-second interview stimulus/response bands, 10–30-word/60-second
prepared-description bands, and 10–30/60–120-second role-play bands.

All Punjabi words, chunks, inflections, and interaction moves must already
appear in lessons of at most 300 seconds. A missing prerequisite is new backlog
work; it is not permission to weaken a task shape or smuggle an untaught model
into the paper.

## Contract and scoring

Add `paper` and `manifest` references only to `a1-mock-1`. Do not implement
`a1-mock-2` in this tranche. Regenerate the Punjabi artifact ceiling so only
the newly present shared rubric and form-1 key cease to count as unbuilt debt.

Reading, listening, writing, and speaking remain independent 100-point papers.
Each must reach 60/100, every section must be attempted, and a high paper cannot
compensate for a failed one. Preserve the exact task-shape weights:

- reading: 35/35/30;
- listening: 35/35/30;
- writing: 40/60; and
- speaking: 30/30/40.

## Security and evidence boundary

The complete learner paper must contain none of the seventeen examiner-only
listening bodies. The test must parse those bodies from the actual confidential
master and prove their absence rather than comparing a second hard-coded list.
The examiner script, key, and scorer-only examples remain outside the learner
packet, and the four skill packets are collected in order so an earlier source
cannot become a later answer model.

This tranche proves artifact presence and structural fidelity only. It does not
prove recording quality, pedagogical quality, a clean administration, standard
setting, qualified Punjabi review, book-only human validation, learner
attainment, exam readiness, accreditation, or equivalence to an external
syllabus.

## Ownership and verification

The focused Punjabi corpus owner pins only this form's files, manifest envelope,
item counts, secure packet order, parsed listening confidentiality, taught-source
boundary, writing stimulus bands, exact selected-response keys, speaking
stimulus and response bands, answer distribution, scoring weights, and evidence
boundary. Generic manifest, artifact, and completion-plan gates own moving
aggregate debt. Form 2 must add its own paper, examiner script, key, manifest,
test, and changelog shard without rewriting this historical owner or the shared
rubric.
