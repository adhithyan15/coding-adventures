# HL53 — Sanskrit A1 first- and second-person object pronouns

## Status

Proposed for issue #17512 under #13415 and #13295.

## Problem

The project-defined Sanskrit A1 inventory point `SA-A1-PRON-03` asks for
object pronouns. Its probe is null because the corpus teaches none of the six
independent first- and second-person accusative forms: **माम्** (*mām*),
**त्वाम्** (*tvām*), **आवाम्** (*āvām*), **युवाम्** (*yuvām*), **अस्मान्**
(*asmān*), and **युष्मान्** (*yuṣmān*). The course already teaches the
transitive frame **पश्यति** (*paśyati*, “sees”), but every existing object in
that frame is a noun. A learner can say that someone sees the house and cannot
yet say that someone sees me, us, or you.

## Contract

Append three Sanskrit-owned chapters after chapter 259. Each chapter advances
one grammatical number and separates first person, second person, and
contrastive retrieval:

1. chapter 260 teaches **माम्**, then **त्वाम्**, then retrieves the singular
   contrast;
2. chapter 261 teaches **आवाम्**, then **युवाम्**, then retrieves the dual
   contrast; and
3. chapter 262 teaches **अस्मान्**, then **युष्मान्**, introduces the
   one/two/several selection rule, and ends with a cumulative retrieval lesson
   that introduces nothing.

The course must not present a paradigm table before the forms have been
learned. It must use already-known `paśyati` frames, so the only new burden is
choosing the object by person and number. The cumulative lesson may place the
six forms in a compact comparison only after all six have been retrieved
individually.

The slice is deliberately bounded to the independent full forms. It does not
teach third-person objects, respectful **भवन्तम्** (*bhavantam*), or the short
enclitic alternatives **मा**, **त्वा**, **नौ**, **वाम्**, **नः**, and **वः**.
The lessons must state that boundary rather than imply that the six full forms
exhaust the Classical Sanskrit pronoun system.

## Knowledge ownership

The ten lessons own exactly nine new atoms:

- `SA-PRON-C260-MAM-01`
- `SA-PRON-C260-TVAM-02`
- `SA-GRAMMAR-C260-SINGULAR-OBJECTS-03`
- `SA-PRON-C261-AVAM-01`
- `SA-PRON-C261-YUVAM-02`
- `SA-GRAMMAR-C261-DUAL-OBJECTS-03`
- `SA-PRON-C262-ASMAN-01`
- `SA-PRON-C262-YUSMAN-02`
- `SA-GRAMMAR-C262-OBJECT-PERSON-NUMBER-03`

Each form atom has exactly one introducing lesson. The two number-local
grammar atoms are introduced in their contrast lessons and revisited by the
chapter-262 cumulative lesson. The all-number selection atom is introduced in
chapter 262's penultimate lesson and revisited by the final lesson. The final
lesson introduces no atom, so every new atom has a later genuine retrieval.

`SA-A1-PRON-03` may map only to these nine atoms and may close only when every
atom exists, is uniquely owned, and the final lesson retrieves the whole
person/number choice.

## Ownership and sharding

Lesson files, chapter ledgers, curriculum extension/path/membership owners,
book targets, generated modality/book/narration owners, corpus evidence, and
the Sanskrit changelog entry must be Sanskrit-owned or single-record additive
files. The inventory point and existing Sanskrit aggregate pins are the only
same-track files whose measured contract changes. No cross-language mutable
aggregate may be reopened.

## Evidence boundary

This slice closes one project-defined editorial A1 content point. It does not
establish B1 human validation, a sourced B1 inventory, A1 or B1 attainment,
exam readiness, learner success, or equivalence to a traditional Sanskrit
examination. The inventory remains explicitly partial.

## Acceptance

- Sanskrit-owned red-before-green tests pin all ten lessons, their order,
  exact atom ownership, prerequisite reachability, duration ceilings, chapter
  payoffs, and the exact inventory probe.
- Every teaching lesson introduces at most one form; the final cumulative
  lesson introduces nothing.
- All six forms are produced in known `paśyati` frames, not merely recognized
  in a list.
- Sanskrit A1 coverage moves from 144/164 to 145/164, with 19 unmapped and
  zero partial points.
- The exact completion plan reports 19 outstanding
  `exam-point/sanskrit/A1` points in four tranches.
- Corpus, curriculum, shard, document-shard, modality, generated-book,
  narration, lesson, figure, compile, and affected-package checks pass.
- Independent security and data-integrity review passes on the exact pushed
  head before every push.
