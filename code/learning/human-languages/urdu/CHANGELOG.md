# Changelog

## 0.8.0 — 2026-08-06

- Added five schema-v2 Chapter 6 micro-lessons, the track's first verbs:
  **ہونا** *honā* (`VERB-BE`), **جانا** *jānā* (`VERB-GO`), **آنا** *ānā*
  (`VERB-COME`), **بولنا** *bolnā* (`VERB-SPEAK`), and **جاننا** *jānnā*
  (`VERB-KNOW`). Urdu had taught zero verbs before this; `SPINE-SAY-WHAT-I-DO`
  is now realized rather than wholly omitted, and the track reaches A2.
- Taught the **-نا** *-nā* infinitive ending as a tool rather than a label:
  strip it and the stem falls out, so each new verb costs less than the last.
- Introduced the two simultaneous agreements the Urdu present makes — the
  participle for gender and number, the copula for person — on *jānā*, where a
  real stem exists to hang them on, and reused the frame unchanged on *bolnā*
  and *jānnā* to show the machine is already built.
- Kept the etymology honest per track: *honā* ← Sanskrit *bhavati* ← PIE
  \**bʰuH-* (English **be**, **build**, **future**, **physics**); *jānnā* ←
  *jñā-* ← \**ǵneh₃-* (English **know**, **notice**, **diagnosis**); *ānā* is
  the *ā-* "toward" preverb welded to the same *yā-* root that hardened into
  *jānā*'s **j-**. *bolnā* is flagged as a genuine dead end — the trail stops
  at Prakrit *bollaï* and the Sanskrit *brūte* link is proposed, not settled.
  *jānā* is flagged as having no English cousin at all rather than being given
  a decorative one.
- Placed the Persian and Arabic literary register beside the Indo-Aryan core on
  *bolnā* (homely *bolnā* against Persian-derived *guftagū*), which is also
  where *nastaʿlīq* is named as part of that same Persian inheritance.
- Cross-language comparison stays self-contained: Hindi is described as the
  other standard form of the same spoken language, never assumed as knowledge
  the reader already has.
- One new letter across the whole chapter: **ب**, taught against already-read
  **پ** as a dot-count contrast. **جاننا**'s doubled **ن** is taught as the
  only thing separating "know" from "go". Added `be-vs-pe-dots` and
  `geminate-nun` to `pronunciation-reference.md`.
- All five lessons derive `voice` modality, so Chapter 6 is fully drivable and
  the track's drivable share rises from 90% to 92%.
- Declared `chapters.json` chapter 6 with `UR-C06-janna` as payoff; measured
  representativeness 9/14 = 0.643, above the 0.5 policy threshold. Generated
  `book/chapters/ch06-core-verbs.tex` and compiled the six-chapter book with
  XeLaTeX: zero `Missing character` warnings, zero errors. Sanskrit forms are
  cited in transliteration only, because this book vendors no Devanagari face.

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
