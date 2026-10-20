# HL63 — Tamil A1 controlled composition

## Status

Proposed for issue #17644 under the Indian-language C2 programme #13295.

## Problem

Tamil has a complete pre-A1 observe, copy, delayed-copy, and dictation runway,
but no model-free `controlled-composition` task. The planner therefore reports
two cumulative Tamil A1 writing gaps: controlled composition and timed
assessment production. This change closes only the first gap. It must not imply
that Tamil has a complete A1 assessment contract or that any learner has reached
A1.

## Required learner experience

Add one chapter-addressable writing lesson after the existing Tamil curriculum.
The lesson must:

- ask the learner to assemble one familiar Tamil location question from meaning;
- reuse only the taught words for **tea** and **where?**;
- show no Tamil answer, word bank, romanization, or copyable model in the
  controlled-composition block;
- make the first attempt untimed and shorter than five minutes;
- introduce no language, script, idiom, sense, or culture knowledge;
- review the thing, place question, word boundary, and punctuation separately;
  and
- state that the single controlled line does not prove A1 attainment, readiness,
  human validation, or completion of the later timed writing task.

## Ownership and publication

The lesson, chapter, curriculum path, extension, membership, book target,
generated book chapter, narration, hashes, modality evidence, test, changelog,
and session-map entry each use their direct shard owner. Ordinary authoring must
not edit a shared track aggregate. Tamil's deliberately absent
`assessment.json` remains absent.

## Verification

A focused Tamil corpus test must prove that exactly one
`controlled-composition` block exists, is untimed, contains the no-model rules,
contains no Tamil learner answer, and uses only the two taught knowledge atoms.
It must also prove that the chapter payoff owns the lesson and that Tamil A1's
only remaining writing gap is `timed-assessment-production`.

Run the Tamil corpus, writing-stage, chapter, curriculum, integration, generated
artifact, document-shard, progress, book, narration, lesson-shard, and package
gates, then run the full affected package suite before the pull request is
marked ready.
