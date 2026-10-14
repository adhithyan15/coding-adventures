# HL54 — Sanskrit A1 first-person future plans

## Status

Proposed for issue #17520 under #13415 and #13295.

## Problem

The project-defined Sanskrit A1 point `SA-A1-F-10` asks the learner to say
what they are going to do. Its note says that the track has no future tense,
but that is no longer wholly true: chapter 139 introduces **भविष्यति**
(*bhaviṣyati*, “will be”) and **गमिष्यति** (*gamiṣyati*, “will go”), and
chapter 140 retrieves them. Those forms only let a learner say what a third
person will do. The A1 mock papers already use first-person forms such as
**गमिष्यामि** (*gamiṣyāmi*) and **आगमिष्यामि** (*āgamiṣyāmi*), although no
lesson introduces a first-person future form.

The point therefore cannot be closed by attaching its probe to the existing
third-person atoms. Recognition of “will go” is not independent production of
“I will go.”

## Contract

Append two Sanskrit-owned chapters after chapter 262:

1. chapter 263 bridges from known **गमिष्यति** to **गमिष्यामि**, names the
   first-person singular ending for this taught form, contrasts “will go” with
   “I will go,” and retrieves that contrast again; and
2. chapter 264 adds **करिष्यामि** (*kariṣyāmi*, “I will do”) and
   **पठिष्यामि** (*paṭhiṣyāmi*, “I will read”), composes a near-term plan with
   known **श्वः** and **परश्वः**, and ends with two successive rounds of
   model-free mixed retrieval.

The ramp teaches complete usable forms before generalisation. It may point out
the shared **-इष्यामि / -िष्यामि** sound-and-shape cue in these three forms,
but it must not present that cue as a complete future-stem algorithm for every
Sanskrit root. The slice is first-person singular only. Second person, dual,
plural, additional future stems, and a full future paradigm remain later work.

## Knowledge ownership

The nine lessons own exactly five new atoms:

- `SA-VERB-C263-GAMISYAMI-01`
- `SA-GRAMMAR-C263-FUTURE-FIRST-SINGULAR-02`
- `SA-VERB-C264-KARISYAMI-01`
- `SA-VERB-C264-PATHISYAMI-02`
- `SA-PATTERN-C264-NEAR-TERM-PLAN-03`

Each atom has one introducing lesson. The two chapter-263 atoms are retrieved
by both later chapter-263 lessons and again in chapter 264. The three
chapter-264 atoms are retrieved by both of the final two lessons. The last two
lessons introduce nothing, so the reinforcement proof is learner work rather
than frontmatter accounting.

The first chapter-263 warm-up also retrieves the preceding chapter's
person-and-number object choice once. Appending this slice otherwise moves
that newly introduced grammar atom beyond its second reinforcement window;
the short handoff keeps the existing A1 gate honest before the topic changes.

`SA-A1-F-10` may map only after all five atoms exist, are uniquely owned, and
the final lesson requires a learner to choose the time word and future form
without a copyable model.

## Ownership and sharding

Lesson files, chapter ledgers, curriculum extension/path/membership owners,
book targets, generated modality/book/narration owners, corpus evidence, and
the Sanskrit changelog entry must be Sanskrit-owned or single-record additive
files. The inventory point and existing Sanskrit aggregate pins are the only
same-track files whose measured contract changes. No cross-language mutable
aggregate may be introduced.

## Evidence boundary

This slice closes one project-defined editorial A1 point. It does not prove
B1 human validation, create the missing B1 inventory, establish exam
readiness, demonstrate learner success, or claim equivalence to a traditional
Sanskrit examination. The A1 inventory remains partial.

## Acceptance

- Sanskrit-owned red-before-green tests pin all nine lessons, their order,
  exact atom ownership, prerequisite reachability, duration ceilings, chapter
  payoffs, and the exact inventory probe.
- The first lesson starts from the already taught third-person future; no
  lesson asks the learner to infer an unseen future stem.
- Every production lesson stays within five minutes, and the final two lessons
  introduce no atoms.
- Learner copy states that this is a first-person singular ramp, not a complete
  future paradigm.
- Sanskrit A1 coverage moves from 145/164 to 146/164, with 18 unmapped and
  zero partial points.
- The exact completion plan reports 18 outstanding
  `exam-point/sanskrit/A1` points in four tranches.
- Corpus, curriculum, shard, document-shard, modality, generated-book,
  narration, lesson, figure, compile, and affected-package checks pass.
- Independent security and data-integrity review passes on the exact pushed
  head before every push.
