# HL50 — Sanskrit A2 connected composition

## Status

Proposed for issue #17480 under #13415 and #13295.

## Problem

Sanskrit's complete source-artifact assessment contract is active, but the
book itself does not yet prove the cumulative `connected-composition` writing
stage first required at A2. The completion planner therefore reports
`writing-stage/sanskrit/A2` even though the A2 assessment envelope and both A2
mock-source bundles exist.

## Contract

Add one language-owned chapter after Sanskrit chapter 257 and one writing
lesson that asks for an untimed first draft of a 70–90-word connected Sanskrit
message or description in Devanagari. The task has a named reader and a
practical purpose, matching the project-defined A2 writing envelope.

The lesson MUST:

1. reuse only already-taught Sanskrit knowledge atoms and introduce no lexical,
   grammatical, script, idiom, sense, or culture claim;
2. carry exactly one `hl-writing-stage: connected-composition` directive;
3. provide a content-only planning scaffold rather than Sanskrit sentences;
4. show no model answer, Sanskrit sentence bank, or romanized answer;
5. exclude dictionaries, translators, generative assistants, and spell-checkers
   from the attempt;
6. stay within the repository's 280-second lesson ceiling; and
7. preserve the original attempt while guiding a separate four-pass review of
   task fulfilment, organisation, Sanskrit language control, and Devanagari
   orthographic control.

The chapter, lesson, curriculum path and extension, curriculum-membership
owner, modality owner, session-map shard, generated book/narration artifacts,
hash owners, tests, and changelog entry MUST remain Sanskrit-owned or
single-record additive files. No cross-language mutable aggregate is reopened.

## Evidence boundary

This slice proves only that the book contains one model-free A2 connected
composition opportunity. It does not prove A2 attainment, a passing mock,
exam readiness, qualified recordings, independent human scoring, standard
setting, book-only validation, learner success, or an external qualification.

## Acceptance

- A Sanskrit-owned red-before-green test pins the lesson, chapter, model-free
  task boundary, already-taught atom set, duration ceiling, and exact writing
  stage.
- `languageWritingStages("sanskrit")` has no defects and no missing A2 stages.
- The exact-merge completion plan no longer emits
  `writing-stage/sanskrit/A2`.
- Corpus, curriculum, shard, document-shard, generated-book, figure, narration,
  lesson, and package checks pass.
- Independent security and data-integrity review passes on the exact pushed
  head before every push.
