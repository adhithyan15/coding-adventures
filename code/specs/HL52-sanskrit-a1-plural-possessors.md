# HL52 — Sanskrit A1 plural possessors

## Status

Proposed for issue #17501 under #13415 and #13295.

## Problem

The Sanskrit A1 editorial inventory point `SA-A1-POS-02` asks for possession
with several possessors: **अस्माकम्** (*asmākam*, “our / of us”) and
**युष्माकम्** (*yuṣmākam*, “your / of you all”). Its probe is null because
neither form occurs in the 1,394-lesson corpus. Chapter 2 teaches only the
singular pairs **मम** (*mama*, “my / of me”) and **तव** (*tava*, “your / of
you”). This is genuine curriculum debt rather than stale measurement.

## Contract

Add one Sanskrit-owned chapter after chapter 258 with three short lessons:

1. teach **अस्माकम्** by contrasting one possessor (`mama`) with several
   possessors (`asmākam`);
2. teach **युष्माकम्** by contrasting one addressee (`tava`) with several
   addressees (`yuṣmākam`); and
3. retrieve all four forms contrastively in already-known noun frames without
   introducing another lexical or grammatical concept.

The chapter must stay on the genitive relationship Sanskrit actually uses. It
must not teach these fixed forms as possessive adjectives that the learner
inflects to agree with the possessed noun, and it must not require the untaught
nominative plural pronouns as a hidden prerequisite. The historical note must
still preserve Whitney's §493 analysis: the forms originated as stereotyped
case-forms of adjective stems even though they function as the genitive plural
forms in the paradigm taught here.

The lessons introduce exactly these atoms:

- `SA-LEX-C259-ASMAKAM-01`
- `SA-LEX-C259-YUSMAKAM-02`
- `SA-GRAMMAR-C259-PLURAL-POSSESSORS-03`

The grammar atom belongs to the contrastive retrieval lesson: it records the
productive one-possessor/several-possessors choice rather than crediting two
isolated vocabulary cards as productive possession.

## Ownership and sharding

The lesson files, chapter ledger, curriculum extension/path/membership owners,
book target, generated modality/book/narration owners, Sanskrit corpus tests,
and Sanskrit changelog entry must be Sanskrit-owned or single-record additive
files. Do not reopen a cross-language mutable aggregate.

## Evidence boundary

This slice closes one project-defined editorial A1 content point. It does not
establish B1 human validation, a sourced B1 inventory, A1 or B1 attainment,
exam readiness, qualified recordings, learner success, or equivalence to a
traditional Sanskrit examination.

## Acceptance

- A Sanskrit-owned red-before-green test pins the three lessons, learner order,
  exact atom ownership, duration ceilings, chapter payoff, and exact inventory
  probe.
- Every lesson is at most 300 seconds and each teaching lesson introduces one
  new form only.
- Sanskrit A1 coverage moves from 143/164 to 144/164, with 20 unmapped and zero
  partial points.
- The exact completion plan reports 20 outstanding
  `exam-point/sanskrit/A1` points.
- Corpus, curriculum, shard, document-shard, modality, generated-book,
  narration, lesson, figure, compile, and affected-package checks pass.
- Independent security and data-integrity review passes on the exact pushed
  head before every push.
