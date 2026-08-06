# Changelog

## 0.7.0 — 2026-08-06

- Added `chapters.json`, the HL05 chapter capability ledger, for Chapters 2–5:
  each declares a first-person `canDo`, the spine nodes it realises, and the
  payoff lesson that proves the claim.
- Every `payoff.assesses` list is the payoff lesson's own
  `practises.knowledge` set verbatim — nothing is claimed that the lesson does
  not already practise, and nothing is padded to clear a threshold.
- Chapter 1 is deliberately omitted. Its four lessons are still schema v1 and
  declare no knowledge atoms, so any payoff written for it would be invented
  rather than derived. The gap is left visible as debt.
- Measured payoff representativeness (assessed ÷ chapter-introduced atoms)
  against the 0.5 policy threshold: ch2 3/3 = 1.00, ch3 8/14 = 0.571,
  ch4 6/16 = 0.375, ch5 4/12 = 0.333. Chapters 4 and 5 sit below threshold
  because their word lessons introduce script, cross-lingual, and etymon atoms
  that the consolidating dialogue does not re-exercise; that is a content gap
  for a later revision, not something to paper over here.
- Chapter capability text describes what the shipped book actually renders. The
  Naskh fallback recorded in HL-U01 remains unfixed, so no chapter claims to
  teach Nastaliq letterforms.

## 0.6.0 — 2026-08-04

- Added four schema-v2 Chapter 5 micro-lessons for **خدا**, **حافظ**, spaced
  **خدا حافظ**, and a start-versus-end interaction.
- Secured the Urdu form before using its Persian and Arabic history as a bridge;
  mixed comparison preserves Urdu spacing and Persian joining.
- Extended the exact N+1/N+3/N+7/N+15 ledger through S35, with objective
  activities and a generated five-chapter book.

## 0.5.0 — 2026-08-04

- Added six schema-v2 Chapter 4 micro-lessons for *kaise/kaisī*, respectful
  **āp ... haiṅ**, the first-person **maiṅ ... hūṅ** frame, *ṭhīk*, the polite
  reply, and cumulative practice.
- Kept addressee agreement separate from honorific register, introduced the
  retroflex-aspiration sequence only inside **ٹھیک**, and made the Hindi bridge
  follow independent Urdu-script retrieval.
- Extended the sound-id reference and exact N+1/N+3/N+7/N+15 ledger through
  S31, with objective activities and a generated four-chapter book.

## 0.4.0 — 2026-08-03

- Added five schema-v2 Chapter 3 micro-lessons for **āp/tum/tū**, *kyā*, the
  full name question, the meeting response, and cumulative practice.
- Added objective activity contracts and prerequisite-closed knowledge atoms for
  the migrated Chapter 2 name frame and every new lesson.
- Generated Chapter 3 for the downloadable book from the same canonical lesson
  AST used by Language Ladder and extended the review ledger through S25.

## 0.3.0 — 2026-08-03

- Added the authoritative five-lesson session map with exact N+1, N+3, N+7,
  and N+15 review placements.
- Added an on-demand Urdu pronunciation and script reference that labels the
  current Naskh presentation fallback without weakening the Nastaliq goal.

## 0.2.0 — 2026-08-02

- Added the first downloadable LaTeX edition.
- Published Chapter 1 (greetings and responses) and Chapter 2 (giving your name)
  from the five dependency-ordered starter lessons.
- Added a B1-oriented track roadmap with Urdu-specific extension points.

## 0.1.0 — 2026-08-02

- Added the Urdu shared-spine pilot with five under-five-minute lessons.
