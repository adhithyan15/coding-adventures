# Coding Adventures Russian Assessment

**Version:** 1.0 target contract, 2026-08-21

**Basis:** project-defined pre-A1 bridge; external TORFL/ТРКИ at A1–C2

**Status:** targets specified; task inventories, mocks, calibration, and
book-only human validation are still backlog

This specification names the examinations that the complete Russian book must
eventually prepare a book-only learner to pass. A1 through C2 target the state
Test of Russian as a Foreign Language (TORFL, Russian ТРКИ). TORFL does not
offer a pre-A1 rung, so the first checkpoint is explicitly project-defined and
non-accredited. The current book is not yet exam-ready.

The checked-in [`assessment.json`](assessment.json) is the machine-readable
contract. It preserves the four-skill floor and declares TORFL **Lexis.
Grammar** as an additional independently required component at A1–C2. Its paths
name required future artifacts; they do not claim that those artifacts exist. A
readiness claim also requires current provider rules, two full timed mocks, and
a successful book-only human pilot.

## External evidence and naming

The Pushkin State Russian Language Institute describes TORFL as a state testing
system with six general-proficiency levels and five subtests: Reading, Writing,
Lexis. Grammar, Listening, and Speaking. Its published mapping is:

| curriculum rung | external target | provider label |
|---|---|---|
| pre-A1 | Coding Adventures Russian pre-A1 Assessment | project-defined bridge; no TORFL certificate |
| A1 | TORFL Elementary Level | ТЭУ / A1 |
| A2 | TORFL Basic Level | ТБУ / A2 |
| B1 | TORFL First Certification Level | ТРКИ-I / B1 |
| B2 | TORFL Second Certification Level | ТРКИ-II / B2 |
| C1 | TORFL Third Certification Level | ТРКИ-III / C1 |
| C2 | TORFL Fourth Certification Level | ТРКИ-IV / C2 |

The Institute publishes typical tests and current regulations; Moscow State
University independently describes the same A1–C2, five-component system. The
provider documents, not this book, remain authoritative for the exam actually
booked. Every task-shape inventory and mock must record the source URL, retrieval
date, and source-file hash used to assemble it. If the provider changes a task,
time, aid, threshold, or retake rule, the affected inventory and mocks must be
versioned before readiness is claimed again.

Sources checked 2026-08-21:

- [Pushkin Institute TORFL system, levels, subtests, and typical tests](https://www.pushkin.institute/certificates/trki/)
- [Pushkin Institute level-by-level sample blocks](https://www.pushkin.institute/certificates/cct/tests-online/)
- [Pushkin Institute 2025 TORFL administration regulations](https://www.pushkin.institute/wp-content/uploads/2025/09/%D0%A0%D0%B5%D0%B3%D0%BB%D0%B0%D0%BC%D0%B5%D0%BD%D1%82-%D0%A2%D0%A0%D0%9A%D0%98.pdf)
- [Moscow State University TORFL A1–C2 overview](https://test.irlc.msu.ru/trki/)
- [CEFR Companion Volume and language versions](https://www.coe.int/en/web/common-european-framework-reference-languages/cefr-companion-volume-and-its-language-versions)

## Pass rule

At pre-A1, reading, listening, writing, and speaking are separate 100-point
papers and each requires **60/100** in the same administration. There is no
aggregate compensation.

At A1–C2, a mock-ready learner must score at least **66% in each of all five
TORFL subtests**: Lexis. Grammar, Reading, Writing, Listening, and Speaking. Some
published TORFL materials describe a limited single-subtest 60% allowance. This
curriculum deliberately does not depend on that allowance: 66% everywhere is a
stable, conservative preparation target that guarantees no stronger component
hides a weak one. The official provider decides the real certificate result.

`assessment.json` encodes Lexis. Grammar in `additionalComponents` at every
external rung. The same requirement must appear in each task inventory, mock
manifest, readiness report, and human-validation report; completion is blocked
when the component is absent or below 66%.

Writing and speaking use the current provider rubric when one is published.
Project-authored practice rubrics must preserve the provider construct and show
their mapping. At minimum, they report task fulfilment, organisation and
interaction, range and control, and Cyrillic orthography for writing or
pronunciation and fluency for speaking. Provider scoring always wins if these
categories differ from the live exam.

## Gentle writing runway

Exam timing never changes the five-minute lesson cap. Writing grows cumulatively:

| first required | learner action |
|---|---|
| pre-A1 | observe and trace one Cyrillic form; copy with a guide; copy after the model is hidden; write heard letters and words |
| A1 | complete constrained forms and short messages; begin timed production only after untimed control is secure |
| A2 | connect sentences for a named reader and purpose |
| B1 | sustain narrative, description, and opinion under a bounded prompt |
| B2 | control register and argument across longer independent texts |
| C1 | synthesise sources and reshape information for an audience |
| C2 | produce precise extended texts across contrasting genres and registers |

Tracing and visible copying are teaching actions, never exam-pass evidence. A
learner advances through many short retrieval steps; only the later mock joins
those skills under continuous official timing.

## Administration and variety

- The assessed variety is contemporary standard Russian in Cyrillic. Documented
  regional or first-language-influenced pronunciation is not penalised merely
  for being non-Moscow when it remains intelligible and satisfies the provider
  rubric.
- Pre-A1 and early A1 directions may include a declared support language during
  instruction. Full TORFL mocks reproduce the live provider's direction and aid
  rules exactly.
- Accessibility accommodations may change presentation or response mode without
  changing the construct. Every accommodation and any provider approval are
  recorded.
- Speaking and writing practice is double-scored by trained Russian raters. A
  third rater adjudicates material disagreements.
- No dictionary, translator, spell-checker, grammar assistant, or generative
  assistant is used unless the current official subtest explicitly permits that
  aid. Permitted aids are recorded per task rather than assumed globally.

## pre-A1

The project-defined bridge lasts 40 minutes in total and establishes the floor
needed to begin official A1 preparation:

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 10 | 6 Cyrillic shape/word matches, 4 short phrase or notice matches, and 4 personal-detail selections; no text over 8 words |
| listening | 10 | 6 sound/word recognitions, 4 greeting-response choices, and 4 personal-detail selections; 70–90 wpm, two plays, 8-second inter-item pauses |
| writing | 12 | 2 delayed-copy items, 4 one-word dictation items, and 2 independently produced personal or greeting responses; no visible model during scoring |
| speaking | 8 | return a greeting, state a name or chosen identity, answer 3 familiar one-turn questions, and make 1 short request; one slow repetition is allowed |

Two unseen project-authored mocks use this same envelope. They are a bridge, not
a TORFL sample, prediction, or certificate.

## A1

Target **ТЭУ / A1 (Elementary Level)**. The provider describes a five-subtest
exam lasting approximately 3 hours 30 minutes. The inventory must transcribe the
current official A1 Reading, Writing, Lexis. Grammar, Listening, and Speaking
parts before mocks are authored. All four cumulative pre-A1 writing stages plus
controlled composition and timed production are required.

## A2

Target **ТБУ / A2 (Basic Level)**, approximately 4 hours across the same five
subtests under the current provider overview. Connected composition joins the
writing evidence. The inventory must preserve the live task counts, source
lengths, recording/replay rules, response limits, timing, and permitted aids.

## B1

Target **ТРКИ-I / B1 (First Certification Level)**, approximately 4 hours 45
minutes. Full mocks cover all five provider subtests in their official order and
timing. They must not substitute a generic CEFR B1 paper for the Russian exam.

## B2

Target **ТРКИ-II / B2 (Second Certification Level)**, approximately 5 hours.
Inventories record the live provider's source genres, interaction conditions,
register demands, aids, and scoring. Writing practice reaches sustained,
audience-aware argument before continuous mock timing begins.

## C1

Target **ТРКИ-III / C1 (Third Certification Level)**, approximately 5 hours 30
minutes. Reading/listening-to-writing integration, professional or academic
discourse, implicit stance, and sustained spoken interaction are included only
where the current official materials require them and are cited per task.

## C2

Target **ТРКИ-IV / C2 (Fourth Certification Level)**, approximately 5 hours 30
minutes. The live provider regulation controls the exact points and times; the
2025 regulation, for example, publishes separate Reading, Writing, Lexis.
Grammar, Listening, and Speaking tables. C2 mocks must reproduce that complete
shape, not merely ask for longer generic essays.

## Required artifacts and readiness language

For every rung, implementation must add:

1. a sourced task inventory for every required component;
2. two complete unseen timed mock forms;
3. provider-aligned rubrics and task-specific scoring notes;
4. answer keys, accepted-response notes, and audio/transcript assets;
5. source hashes plus a documented provider-drift review;
6. rater training, double-scoring, and adjudication evidence;
7. a pre-registered book-only human-validation report.

For A1–C2, “every component” includes Lexis. Grammar. Until items 1–4 exist,
reports may say **target contracted** only. After a learner passes both complete
mocks they may say **mock ready**. Only the human study can support **learner
proven**. The actual TORFL certificate is awarded only by an authorised testing
provider.
