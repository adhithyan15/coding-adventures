# HL61 — Sanskrit A1 person and quiet requests

## Status

Proposed for issue #17573 under #13415 and #13295.

## Problem

The project-defined Sanskrit A1 point `SA-A1-F-22` says that neither asking
for a person nor asking somebody to be quiet has a taught exponent. That note
is stale. The corpus already owns **कुत्र** (*kutra*, “where?”) as
`SA-LEX-C14-DEIXIS-06`, **अस्ति** (*asti*, “is”) as `SA-LEX-ASTI`, familiar
person roles including **अध्यापकः** (*adhyāpakaḥ*, “teacher”) as
`SA-LEX-C83-SCHOOL-03`, and **कृपया** (*kṛpayā*, “please”) as
`SA-LEX-C29-COURTESY-01`.

The remaining work is small but real: compose existing owners into a bounded
availability exchange for a familiar person, compose one polite request for
quiet from a sourced command plus already-owned **कृपया**, and retrieve both
acts without a visible answer model.

## Source boundary

The quiet request is the composition **कृपया तूष्णीं भव** (*kṛpayā tūṣṇīṃ
bhava*, “please be quiet”). The new sourced material is limited to the
attested command **तूष्णीं भव**:

- the Board of School Education Haryana copy of the NCERT Class 10
  *Śemuṣī* Sanskrit reader uses **तूष्णीं भव** as a direct command in the
  chapter “सौहार्दं प्रकृतेः शोभा”; and
- the grammatical commentary on Pāṇini 3.4.63, **तूष्णीमि भुवः**, explicitly
  treats **तूष्णीम्** with √भू.

The already-owned **कृपया** softens that exact command. This contract does not
generalise from **भव** to the imperative paradigm or from one request to
general imperative mastery.

## Contract

Append one Sanskrit-owned chapter after chapter 270 with four lessons:

1. compose the doorway/telephone availability exchange **अध्यापकः अत्र
   अस्ति? — आम्, अध्यापकः अत्र अस्ति।** (“Is the teacher here?” — “Yes, the
   teacher is here.”) from existing owners as the bounded ask-for-a-person and
   response turn;
2. teach the composed request **कृपया तूष्णीं भव** (“Please be quiet”), using
   the sourced command **तूष्णीं भव** plus already-owned **कृपया**;
3. retrieve both acts from English cues without a visible Sanskrit model; and
4. retrieve both acts again in reverse order, again without a visible model.

The first lesson introduces only the composed person-request pattern. The
second introduces only the bounded quiet-request pattern. The retrieval
lessons introduce nothing. The second retrieval lesson is the chapter payoff
and the exact inventory probe.

## Knowledge ownership

The chapter owns exactly two new atoms:

- `SA-PATTERN-C271-ASK-FOR-PERSON-01`
- `SA-PATTERN-C271-REQUEST-QUIET-02`

The exact `SA-A1-F-22` probe is those two atoms. Existing lessons remain the
sole owners of the component words. The quiet-request atom owns the complete
composed request. Its sources attest only **तूष्णीं भव** and the **तूष्णीम्**
+ √भू collocation; already-owned **कृपया** supplies the polite softener. The
atom does not create separate lexical or paradigm ownership for **तूष्णीम्**
or **भव**.

## Model-free response boundary

Both retrieval lessons must present the English cues, then an eight-second
response window, and only then a separate answer-check block. Neither target
in Devanagari nor either romanization may occur anywhere before that answer
window in the authored lesson or generated narration.

## Ownership and sharding

Lesson files, chapter ledger, curriculum extension/path/membership owners,
book target, generated modality/book/narration owners, corpus evidence, and
the Sanskrit changelog entry must be Sanskrit-owned or single-record additive
files. The one inventory point and existing Sanskrit aggregate pins are the
only same-track files whose measured contract changes. No cross-language
mutable aggregate may be introduced.

## Evidence boundary

This slice closes one project-defined editorial A1 point. It proves two
model-free retrievals of two bounded spoken acts. It does not prove general
imperative mastery, conversational competence, learner attainment, human
validation, or exam readiness. The A1 inventory remains partial.

## Acceptance

- A Sanskrit-owned red-before-green test pins all four lessons, their order,
  exact ownership, duration ceilings, source links, chapter payoff, and exact
  inventory probe.
- Both retrieval lessons introduce nothing and expose no target Sanskrit
  model before the learner answers.
- Existing component lessons remain their sole owners.
- Sanskrit A1 coverage moves from 153/164 to 154/164, with 10 unmapped and
  zero partial points.
- Corpus, curriculum, shard, document-shard, modality, generated-book,
  narration, lesson, affected-package, and full-book checks pass.
- Independent security and data-integrity review passes on the exact pushed
  head before every push.
