# HL64 — Tamil A1 timed writing checkpoint

## Status

Proposed for issue #17651 under the Indian-language C2 programme #13295.

## Problem

Tamil's A1 writing runway now reaches model-free controlled composition, but it
does not yet ask the learner to retrieve familiar writing under a clock. The
planner therefore reports `timed-assessment-production` as Tamil A1's sole
remaining enumerable writing-stage gap. This change closes that curriculum
rung without pretending that the separate full A1 assessment exists.

## Required learner experience

Add one chapter-addressable writing lesson after chapter 281. The lesson must:

- give the learner three minutes to complete six short fields in Tamil script;
- reuse only already-taught material for a greeting, name, language, tea,
  location question, and closing;
- show no Tamil answer, model, word bank, or romanization in the timed block;
- prohibit dictionaries, translators, and spell-checkers;
- stop the learner when the clock rings and preserve unfinished work as honest
  evidence;
- introduce no language, script, idiom, sense, or culture knowledge; and
- state that this short checkpoint is not the full A1 paper and proves neither
  attainment, readiness, completeness, nor human validation.

## Assessment boundary

Tamil's `assessment.json` remains deliberately absent. This lesson must not
link to nonexistent mocks, rubrics, or keys. The eventual A1 paper remains
blocked behind the inventories and assessment artifacts named in
`tamil/assessment-spec.md`; the lesson is only a gentle timed curriculum rung.

## Ownership and publication

The lesson, chapter, curriculum path, extension, membership, book target,
generated book chapter, narration, hashes, modality evidence, test, changelog,
and session-map entry each use their direct shard owner. Ordinary authoring must
not edit a shared track aggregate.

## Verification

A focused Tamil corpus test must prove that exactly one
`timed-assessment-production` block exists, follows the controlled-composition
lesson, stays within the under-five-minute budget, contains the independent
writing rules, and contains no Tamil answer. It must also prove that the chapter
payoff owns the lesson, the Tamil A1 writing-stage gap is closed, and
`assessment.json` remains absent.
