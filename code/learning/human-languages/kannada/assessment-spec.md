# Coding Adventures Kannada Assessment

**Version:** 1.0 target contract, 2026-09-13

**Basis:** project-defined CEFR-aligned equivalent

**Status:** target specified; the pre-A1 task inventory is checked in, while
A1-C2 inventories, mocks, calibration, and human validation remain backlog

This specification names the assessment that the complete Kannada book must
eventually prepare a book-only learner to pass. It is not an external
qualification, and it does not claim that the current book is exam-ready. The
learner-facing name at each rung is **Coding Adventures Kannada <Level>
Assessment — project-defined equivalent**.

No `assessment.json` is checked in for Kannada yet, and that omission is
deliberate. The machine-readable contract names required artifacts by path, and
the artifact gate treats a path that leads nowhere as an error rather than as a
promise. The contract is written after the inventories, mocks, rubrics and
answer keys it points at, not before them.

## Evidence and scope

The proficiency backbone is the Council of Europe's 2020 CEFR Companion Volume,
including its pre-A1 descriptors and its separate scales for reception,
production, interaction, mediation, and written production. CEFR is a framework
for describing proficiency and developing transparent examinations; it is not a
Kannada awarding body, and the Council of Europe states that it does not verify
or validate an examination's claimed link to CEFR. Every target here is
therefore labelled `project-defined`.

Kannada has a real graded ladder with real consequences attached. The **Kannada
Sahitya Parishat** conducts four examinations — **Kannada Pravesha**, **Kannada
Kava**, **Kannada Jaana** and **Kannada Ratna** — and a state government
employee who clears them is exempted from their own department's Kannada test.
That is not a hobbyist certificate; it decides whether someone keeps a
qualification requirement hanging over them.

It is also, precisely for that reason, **not this book's ladder**. It is aimed
at state government employees and schoolchildren inside Karnataka: people who
already live in the language and need to certify literacy in it, rather than
someone starting from no Kannada at all somewhere else. It publishes no
per-skill structure and no pass rule, so there is nothing to inherit even if the
audience matched. This project defines its own transparent approximation and
states the rule in the open.

The assessed variety is contemporary standard Kannada in the Kannada script. The
current track's taught forms define the initial model. Documented regional
forms receive credit when used consistently and when they satisfy the task; a
trained Kannada reviewer adjudicates unfamiliar forms.

Sources checked 2026-09-13:

- [CEFR Companion Volume and language versions](https://www.coe.int/en/web/common-european-framework-reference-languages/cefr-companion-volume-and-its-language-versions)
- [Council of Europe CEFR descriptors](https://www.coe.int/en/web/common-european-framework-reference-languages/cefr-descriptors)
- [Council of Europe guidance on tests and examinations](https://www.coe.int/en/web/common-european-framework-reference-languages/tests-and-examinations)
- [Council of Europe introduction and certification caveat](https://www.coe.int/en/web/common-european-framework-reference-languages/introduction-and-context)
- [Kannada Sahitya Parishat's Kannada Exams — Star of Mysore](https://starofmysore.com/kannada-sahitya-parishats-kannada-exams/)

## Pass rule

Reading, listening, writing, and speaking are separate 100-point papers. A
candidate must score at least **60/100 on every paper in the same
administration**. There is no aggregate compensation: 90 in reading cannot hide
59 in writing. An unattempted required part scores zero. A complete pass also
requires both published timed mocks at the rung to have been passed under the
same independent thresholds before the book may call the learner mock ready.

Writing and speaking use four equally weighted analytic dimensions:

1. task fulfilment and relevant content;
2. comprehensibility, organisation, and interaction;
3. range and control of Kannada vocabulary and grammar;
4. Kannada orthographic control for writing, or pronunciation and fluency for
   speaking.

Each dimension is scored 0–5, then scaled to the paper's points. A response that
is not meaningfully in Kannada cannot earn task-fulfilment credit.

Dimension 4 for writing is scored on three things the Kannada writing system
actually requires:

- the **ಒತ್ತಕ್ಷರ** (*ottakshara*), the subscript consonant. Kannada writes a
  conjunct by hanging a reduced second consonant BELOW the base letter rather
  than beside it, so a line of Kannada occupies three tiers and a reader's eye
  has to travel down as well as along. The corpus builds 1,488 of them, and
  writing one beside the base instead of under it is an error of a kind a
  Latin-script hand has no practice at;
- the **rounded letterform.** Kannada rounds nearly every letter because the
  script was cut into palm leaf for a thousand years and a straight stroke
  splits a leaf along its grain. A hand that straightens the curves is not
  making a stylistic choice — the rounded shapes are what make letters
  distinguishable at a glance, which is most of what reading a word cold is;
- **vowel-sign placement and legibility.**

## The writing ramp, measured rather than assumed

Kannada has **50 script lessons** in its learning record and **not one
writing-stage directive** in any of them. Observe/trace, guided copy, delayed
copy and dictation/transcription are unproved for this track — not partly
proved, unproved — while the pre-A1 writing paper below requires delayed recall
and dictation.

The inventory describes the examination, not the corpus, and the gap between the
two documents is the work. Naming it is what makes it fixable; the alternative
is a contract that reads as satisfied by fifty lessons that do not stage
anything.

## Administration rules

- Listening recordings use Kannada speakers. Words-per-minute bands below are
  form-assembly targets, not a claim that Kannada has one natural speaking rate.
- Pre-A1 and A1 directions may also be supplied in a declared support language,
  so misunderstanding directions is not confused with Kannada proficiency. From
  A2 onward directions are in Kannada, with one unscored example.
- No dictionary, grammar reference, spell-checker, translator, or generative
  assistant is allowed. Ordinary accessibility accommodations may change
  presentation or response mode without changing the construct; every change is
  recorded.
- Listening pause and replay rules are fixed below. A technical restart replaces
  the affected recording; it is not an extra candidate-selected replay.
- Speaking is recorded and independently scored by two trained raters. A third
  rater adjudicates when scaled totals differ by more than 10 points or when the
  first two raters disagree about a documented regional form.
- Full mocks use unseen prompts and the target timing. Five-minute lessons build
  the skills gently; full assessments keep continuous timing because they are
  evidence, not instruction.

The 60% threshold is a transparent provisional project standard, not validation
evidence. Before release, at least eight qualified Kannada educators or
linguists must perform a modified Angoff review; two raters must double-score at
least 50 writing and 50 speaking samples per rung; and a book-only pilot must
report every skill separately. The contract is versioned if standard setting
changes the threshold or task envelope.

## Level envelopes

The later `task-shapes/<level>.json` inventories must make every part below
executable without expanding its input, response, timing, replay, interaction,
or aid boundary. The A1-C2 bands are the project's own shared ladder, identical
across tracks by design: a level means the same amount of language everywhere,
and only the script-specific rules above differ per track.

### pre-A1

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 10 | 6 Kannada shape/word matches, 4 short phrase or notice matches, and 4 personal-detail selections; no text over 8 words |
| listening | 10 | 6 sound/word recognitions, 4 greeting-response choices, and 4 personal-detail selections; 70–90 wpm, two plays, 8-second inter-item pauses |
| writing | 12 | 2 delayed-copy items, 4 one-word dictation items, and 2 independently produced personal or greeting responses; no visible answer model during scoring |
| speaking | 8 | return a greeting, state a name or chosen identity, answer 3 familiar one-turn questions, and make 1 short request; the interlocutor may repeat once slowly |

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
| speaking | 12 | interview, 2-minute picture or topic account, and a role-play requiring information exchange; 5 minutes preparation |

### B1

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 45 | 4 parts and 26–32 items across correspondence, public information, narrative, and an informational article; 1,100–1,400 source words total |
| listening | 35 | 4 parts and 22–28 items across transactions, interviews, narrative, and a short talk; 130–150 wpm, two plays for short items and one replay for each long item |
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
| speaking | 22 | 5-minute source-based presentation, collaborative synthesis, and sustained challenge and defence; 15 minutes preparation with paper notes only |

### C2

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 90 | 4 parts and 34–42 items across stylistically varied specialist, public, and literary texts; 4,000–5,000 source words, including ambiguity and intertextual stance |
| listening | 60 | 4 parts and 28–34 items across rapid multiparty interaction, extended argument, narrative, and culturally dense media; natural 165–200 wpm variation, recordings played once |
| writing | 90 | synthesize and reframe multiple sources for one audience in 220–280 words, then produce a precise 420–520 word extended text in a contrasting genre or register |
| speaking | 25 | 6-minute synthesis or mediation presentation, multiparty negotiation, and sustained defence requiring reformulation for a second audience; 15 minutes preparation with paper notes only |

## Required artifacts and readiness language

For each rung, implementation must add:

1. the four-skill JSON task inventory named by `assessment.json`;
2. two complete timed mock forms with fresh input and prompts;
3. a shared analytic rubric plus task-specific scoring notes;
4. one answer key per mock, including acceptable regional alternatives;
5. rater training samples, double-scoring evidence, and adjudication records;
6. the pre-registered book-only human-validation report.

Until items 1–4 exist, reports may say **target contracted** only. After a
learner passes both mocks they may say **mock ready**. Only the pre-registered
human study can support **learner proven**, and its result must never be inferred
from corpus coverage.
