# HL81 — Punjabi A2 transport prerequisite

## Status

Implementation plan for issue #17789. This is a fourth taught-content
prerequisite for Punjabi A2 mock 1 (#17739), not a mock or readiness evidence.

## Measured dependency

After PR #17843 merged, exact-main closure at
`f75fe19ee90e54775743b8bb60c61b3d3d32f1e2` left 267 distinct unsupported
Gurmukhi forms and 487 combined occurrences in the preserved learner paper and
confidential examiner script. The next bounded semantic group is **ਰੇਲ** (7),
**ਟਿਕਟ** (4), **ਯਾਤਰਾ** (4), **ਵਾਪਸ** (4), and **ਪੈਦਲ** (3): 22 occurrences.

## Scope and sequence

Add chapter 181 with eight lessons of at most 300 seconds. A short recognition
lesson introduces **ਯ** immediately before its first use in **ਯਾਤਰਾ**; no
stroke order is claimed without evidence. The five vocabulary lessons
introduce exactly one of the measured forms each. An atom-free review
and an atom-free four-skill payoff give every atom at least two later
retrievals. Build on already taught **ਸਟੇਸ਼ਨ**, **ਗੱਡੀ**, **ਘਰ**, **ਜਾਣਾ**,
**ਮੈਨੂੰ ... ਪਸੰਦ ਹੈ**, and **ਨਾਲ** rather than opening a new grammar table.

The bounded payoff is:

> **ਰੇਲ ਦੀ ਟਿਕਟ ਇੱਥੇ ਹੈ। ਮੈਨੂੰ ਰੇਲ ਨਾਲ ਯਾਤਰਾ ਪਸੰਦ ਹੈ। ਮੈਨੂੰ ਪੈਦਲ ਵਾਪਸ ਘਰ ਜਾਣਾ ਪਸੰਦ ਹੈ।**

Learners hear and say the note, read it, then write it only after covering the
model. Supporting Punjabi must occur before chapter 181. Keep the opening
voice-drivable and preserve direct chapter, curriculum, membership, session,
modality, narration, book, hash, test, spec, and changelog ownership.

## Acceptance

The focused owner pins exact lesson order and duration, one owner per new
atom, source-before-use ordering (including **ਯ**), two later retrievals, supporting-token
closure, the four-skill payoff, the measured 22-occurrence benefit, and honest
non-readiness boundaries. Run normal shard, documentation-shard, narration,
modality, book-generation, assessment-artifact, focused corpus, and package
checks.

## Evidence boundary

This chapter does not publish or unblock Punjabi A2 mock 1 by itself. It does
not establish A2 readiness, attainment, equivalence, recording quality,
standard setting, or qualified-human validation.

## Language evidence used

Punjabi University's RCPLT Punjabi–English dictionary supports all five
current Punjabi meanings. The established root registry supports **ਰੇਲ** as
an English *rail* loan, **ਟਿਕਟ** as an English *ticket* loan, and **ਯਾਤਰਾ** as
the learned Sanskrit *yātrā* form. No origin claim is made for **ਵਾਪਸ** or
**ਪੈਦਲ** in this bounded chapter.
