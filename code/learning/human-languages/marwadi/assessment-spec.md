# Coding Adventures Marwadi Assessment

**Version:** 1.0 target contract, 2026-08-20

**Basis:** project-defined CEFR-aligned equivalent

**Status:** target specified; task inventories, mocks, calibration, and human
validation are still backlog

This specification names the assessment that the Marwadi book must eventually
prepare a book-only learner to pass. It is not an external qualification and it
does not claim that the current starter chapter is exam-ready. The learner-facing
name at each rung is **Coding Adventures Marwadi <Level> Assessment —
project-defined equivalent**.

The checked-in [`assessment.json`](assessment.json) is the machine-readable
contract. Paths in that file name required future artifacts. A path is a
dependency, not evidence that the task inventory, rubric, answer key, or mock
already exists. Those assets, two timed mock passes, and a book-only human pilot
must all land before a readiness claim is permitted.

## Evidence and scope

The proficiency backbone is the Council of Europe's 2020 CEFR Companion Volume,
including its pre-A1 descriptors and its separate scales for reception,
production, interaction, mediation, and written production. CEFR is a framework,
not a Marwadi awarding body, so every target in this contract is labelled
`project-defined`.

The initial assessed variety is contemporary central Marwari written in
Devanagari. Government of Rajasthan material locates Marwari in western
Rajasthan, and Census of India tables identify Marwari as a reported mother
tongue. Neither source establishes one exclusive standard. Valid documented
regional forms therefore receive credit when the candidate uses them
consistently; a trained Marwari reviewer adjudicates a form that a rater does not
recognise. Hindi replacement is not silently accepted as Marwari, and natural
code-switching is assessed only when a task explicitly licenses it.

Sources checked 2026-08-20:

- [CEFR Companion Volume (2020)](https://rm.coe.int/cefr-companion-volume-with-new-descriptors-2020/16809ea0d4)
- [Council of Europe CEFR level descriptions](https://www.coe.int/en/web/common-european-framework-reference-languages/level-descriptions)
- [Government of Rajasthan: Know Rajasthan](https://chms.rajasthan.gov.in/Home/Know_Rajasthan)
- [Census of India C-16 mother-tongue catalogue](https://censusindia.gov.in/nada/index.php/catalog/22848)

## Pass rule

Reading, listening, writing, and speaking are separate 100-point papers. A
candidate must score at least **60/100 on every paper in the same
administration**. There is no aggregate compensation: 90 in reading cannot hide
59 in writing. Unattempted required parts score zero. A complete pass also
requires both published timed mocks at the rung to have been passed under the
same independent thresholds before the book may describe the learner as mock
ready.

Writing and speaking use four equally weighted analytic dimensions:

1. task fulfilment and relevant content;
2. comprehensibility, organisation, and interaction;
3. range and control of Marwari vocabulary and grammar;
4. orthographic control for writing, or pronunciation and fluency for speaking.

Each dimension is scored 0–5, then scaled to the paper's points. A response that
is not meaningfully in Marwari cannot earn task-fulfilment credit. Documented
regional vocabulary, morphology, pronunciation, and spelling variants are not
errors merely because they differ from the central-variety model.

The 60% threshold is a transparent provisional standard, not validation
evidence. Before release, at least eight qualified Marwari educators or
linguists must perform a modified Angoff review, two raters must double-score at
least 50 writing and 50 speaking samples per rung, and a book-only pilot must
report each skill separately. The contract is versioned if standard setting
changes the threshold or task envelope.

## Administration rules

- Listening recordings use Marwari speakers. The stated words-per-minute bands
  are form-assembly targets, not claims about one natural speaking rate.
- Pre-A1 and A1 instructions may also be supplied in a declared support language
  so misunderstanding the test directions is not confused with Marwari ability.
  From A2 onward, task directions are in Marwari, with one unscored example.
- No dictionary, grammar reference, spell-checker, translator, or generative
  assistant is allowed. Ordinary accessibility accommodations may change
  presentation or response mode without changing the construct; every change is
  recorded.
- Listening pause/replay rules are fixed below. A technical restart replaces the
  affected recording; it is not an extra candidate-selected replay.
- Speaking is recorded and independently scored by two trained raters. A third
  rater adjudicates a paper when the first two scaled totals differ by more than
  10 points or disagree about whether a regional form is valid.
- Full mocks use unseen prompts and the target timing. Five-minute lessons build
  the skills, but a full assessment keeps authentic continuous timing because it
  is evidence rather than instruction.

## Level envelopes

The later `task-shapes/<level>.json` inventories must make every part below
executable without expanding its input, response, timing, replay, interaction,
or aid boundary.

### pre-A1

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 10 | 6 sign/word matches, 4 short phrase or notice matches, and 4 personal-detail selections; no text over 8 words |
| listening | 10 | 6 sound/word recognitions, 4 greeting-response choices, and 4 personal-detail selections; 70–90 wpm, two plays, 8-second inter-item pauses |
| writing | 12 | 2 delayed-copy items, 4 one-word dictation items, and 2 independently produced personal/greeting responses; no visible answer model during scoring |
| speaking | 8 | return a greeting, state a name or chosen identity, answer 3 familiar one-turn questions, and make 1 short request; trained interlocutor may repeat once slowly |

Writing proves observe/trace, guided copy, delayed copy, and
dictation/transcription in the learning record. Only delayed recall, dictation,
and independent production earn paper points; tracing and visible copying remain
instructional supports, not pass evidence.

### A1

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 20 | 3 parts and 18–22 items across signs, forms, messages, and personal descriptions; 250–350 source words total, no text over 90 words |
| listening | 18 | 3 parts and 15–18 items across announcements, short exchanges, and a personal account; 90–110 wpm, two plays |
| writing | 20 | complete a practical form, then write a 30–40 word message for a named reader and purpose |
| speaking | 10 | personal interview, 1-minute prepared description, and a simple transactional role-play; 5 minutes preparation |

### A2

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 30 | 3 parts and 22–26 items across notices, correspondence, and two connected everyday texts; 550–750 source words total |
| listening | 25 | 3 parts and 18–22 items across announcements, conversations, and one 2–3 minute account; 110–130 wpm, two plays |
| writing | 30 | a 25–35 word functional response and a 70–90 word connected message or description |
| speaking | 12 | interview, 2-minute picture/topic account, and a paired or interlocutor role-play requiring information exchange; 5 minutes preparation |

### B1

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 45 | 4 parts and 26–32 items across correspondence, public information, narrative, and an informational article; 1,100–1,400 source words total |
| listening | 35 | 4 parts and 22–28 items across transactions, interviews, narrative, and a short talk; 130–150 wpm, two plays for short items and one replay of each long item |
| writing | 45 | a 50–70 word functional text and a 130–170 word connected narrative, description, or opinion for a named audience |
| speaking | 15 | interview, 3-minute prepared account, collaborative planning task, and follow-up discussion; 10 minutes preparation |

### B2

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 60 | 4 parts and 30–36 items across argument, reporting, instructions, and literary or cultural prose; 1,800–2,300 source words total |
| listening | 45 | 4 parts and 24–30 items using multiple speakers and at least two regional voices; 150–170 wpm, long recordings played once after one orienting preview |
| writing | 60 | a 100–130 word interaction text and a 220–280 word report, article, narrative, or reasoned argument in the required register |
| speaking | 18 | 4-minute presentation, collaborative problem-solving, and defended discussion with follow-up questions; 10 minutes preparation |

### C1

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 75 | 4 parts and 32–40 items across dense public, professional, academic, and literary texts; 2,800–3,500 source words total, including implicit stance |
| listening | 50 | 4 parts and 26–32 items across unscripted discussion, lecture, interview, and narrative; 160–185 wpm with natural variation, recordings played once |
| writing | 75 | synthesize two short sources in 180–220 words, then produce a 300–380 word audience-aware argument, report, or critical response |
| speaking | 22 | 5-minute source-based presentation, collaborative synthesis, and sustained challenge/defence; 15 minutes preparation with paper notes only |

### C2

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 90 | 4 parts and 34–42 items across stylistically varied specialist, public, and literary texts; 4,000–5,000 source words, including ambiguity and intertextual stance |
| listening | 60 | 4 parts and 28–34 items across rapid multiparty interaction, extended argument, narrative, and culturally dense media; natural 165–200 wpm variation, recordings played once |
| writing | 90 | synthesize and reframe multiple sources for one audience in 220–280 words, then produce a precise 420–520 word extended text in a contrasting genre or register |
| speaking | 25 | 6-minute synthesis/mediation presentation, multiparty negotiation, and sustained defence that requires reformulation for a second audience; 15 minutes preparation with paper notes only |

## Required artifacts and readiness language

For each rung, implementation must add:

1. the four-skill JSON task inventory named by `assessment.json`;
2. two complete timed mock forms with fresh input and prompts;
3. a shared analytic rubric plus task-specific scoring notes;
4. one answer key per mock, including acceptable regional alternatives;
5. rater training samples, double-scoring evidence, and adjudication records;
6. the pre-registered book-only human-validation report.

Until items 1–4 exist, reports may say **target contracted** only. After a learner
passes both mocks they may say **mock ready**. Only the pre-registered human study
can support **learner proven**, and its result must never be inferred from corpus
coverage.
