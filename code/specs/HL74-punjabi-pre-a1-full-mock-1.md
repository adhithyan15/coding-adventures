# HL74 — Punjabi pre-A1 full mock 1

## Status

Proposed for issue #17712 under the Punjabi bridge #13384 and the
Indian-language C2 programme #13295.

## Problem

Punjabi has a complete project-defined pre-A1 task-shape inventory and names
two timed forms in `assessment.json`, but neither form has an executable
learner paper. A timed declaration, a missing key, and a missing shared rubric
are promises rather than assessment evidence.

## Required artifact set

Make `pre-a1-mock-1` the first structurally complete Punjabi mock by adding:

- one learner paper covering all fourteen reading items, fourteen listening
  items, eight writing items, and six speaking prompts in the published
  10/10/12/8-minute envelope;
- one confidential examiner and recording script that keeps listening,
  dictation, and live-interlocutor stimuli away from the learner paper;
- one form-specific answer key;
- one shared pre-A1 rubric that can score either declared form without a later
  edit; and
- one version-1 manifest binding exactly one visible span in the paper, rubric,
  and key to every published task-part ID.

The learner paper uses Gurmukhi for Punjabi stimuli and supplies no romanized
answer aid. Every tested word, chunk, and interaction must already be taught in
the Punjabi course. Productive items may accept equivalent taught responses;
an answer-key example is guidance for scorers, not a script the learner must
reproduce.

## Contract and scoring

Add `paper` and `manifest` references only to `pre-a1-mock-1`. Do not implement
`pre-a1-mock-2` in this tranche; it remains the next dependent item. Regenerate
the Punjabi artifact ceiling so the newly present shared rubric and mock-1
answer key stop appearing as unbuilt debt.

Reading, listening, writing, and speaking remain independent 100-point papers.
Each must reach 60/100; a strong paper cannot compensate for a failed one. The
rubric preserves the exact published part weights:

- reading and listening: 40/30/30;
- writing: 20/40/40; and
- speaking: 30/40/30.

## Evidence boundary

This tranche proves artifact presence and structural fidelity only. It does
not prove pedagogical quality, a clean administration, standard setting,
qualified Punjabi review, book-only human validation, learner attainment,
exam readiness, accreditation, or equivalence to an external syllabus.

## Ownership and verification

The focused Punjabi corpus owner pins only this mock's declared files, exact
manifest envelope, item counts, taught-source boundary, secure packet order,
confidential targets, duration bands, answer distribution, and evidence
boundary. Generic manifest, artifact, and completion-plan gates own the moving
aggregate and missing-mock debt, so the follow-up form never rewrites this
historical owner. The follow-up mock must add its own paper, key, manifest,
examiner script, test, and changelog owner without rewriting this form or the
shared rubric.
