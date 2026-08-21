# Coding Adventures Punjabi Assessment

**Version:** 1.0 target contract, 2026-08-21

**Basis:** project-defined CEFR-aligned equivalent

**Status:** target specified; task inventories, mocks, calibration, and human
validation are still backlog

This specification names the assessment that the complete Punjabi book must
eventually prepare a book-only learner to pass. It is not an external
qualification, and it does not claim that the current book is exam-ready. The
learner-facing name at each rung is **Coding Adventures Punjabi <Level>
Assessment — project-defined equivalent**.

The checked-in [`assessment.json`](assessment.json) is the machine-readable
contract. Its paths name required future artifacts. A path is a dependency, not
evidence that a task inventory, rubric, answer key, or mock already exists. All
of those assets, two timed mock passes, and a book-only human pilot must land
before a readiness claim is permitted.

## Evidence and scope

The proficiency backbone is the Council of Europe's CEFR Companion Volume. It
adds pre-A1 descriptors and separates reception, production, interaction,
mediation, and written production. CEFR is a framework for describing
proficiency and building transparent assessments; it is not a Punjabi awarding
body. The Council of Europe also says that it does not verify an examination's
claimed link to CEFR. No external Punjabi examination has been selected as a
complete pre-A1-to-C2, four-skill destination for this track, so every target is
honestly labelled `project-defined`.

The assessed variety is the contemporary Punjabi taught by this track, written
in Gurmukhi. The Government of Punjab identifies Punjabi as the state's official
language and Gurmukhi as its script. That establishes the writing convention for
this book; it does not confer accreditation on this assessment. Shahmukhi is an
important Punjabi writing tradition but is outside this track's present script
contract. A future Shahmukhi track must specify and validate its own writing
path rather than silently mixing scripts inside a scored response.

Documented regional Punjabi vocabulary, morphology, pronunciation, and tone
patterns receive credit when used consistently and when they satisfy the task.
A trained Punjabi reviewer adjudicates unfamiliar forms. Natural code-switching
is assessed only when a task explicitly licenses it; replacing the required
Punjabi performance with another language does not earn task-fulfilment credit.

Sources checked 2026-08-21:

- [CEFR Companion Volume and language versions](https://www.coe.int/en/web/common-european-framework-reference-languages/cefr-companion-volume-and-its-language-versions)
- [Council of Europe CEFR descriptors](https://www.coe.int/en/web/common-european-framework-reference-languages/cefr-descriptors)
- [Council of Europe guidance on tests and examinations](https://www.coe.int/en/web/common-european-framework-reference-languages/tests-and-examinations)
- [Council of Europe examination-linking caveat](https://www.coe.int/en/web/common-european-framework-reference-languages/relating-examinations-to-the-cefr)
- [Government of Punjab: Punjabi and Gurmukhi](https://punjab.gov.in/know-punjab/)

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
3. range and control of Punjabi vocabulary and grammar;
4. Gurmukhi orthographic control for writing, or pronunciation, tone, and
   fluency for speaking.

Each dimension is scored 0–5, then scaled to the paper's points. At pre-A1 and
A1, a tone or spelling error is penalised only when it obscures the intended
word or the task directly targets that contrast. From A2 upward, repeated
uncontrolled contrasts affect language-control points while communication
remains separately credited. Documented regional forms are not errors merely
because they differ from the form used in the book. Gendered first-person forms
are judged for consistent agreement, not against an assumed candidate identity.

The 60% threshold is a transparent provisional project standard, not validation
evidence. Before release, at least eight qualified Punjabi educators or
linguists must perform a modified Angoff review; two raters must double-score at
least 50 writing and 50 speaking samples per rung; and a book-only pilot must
report every skill separately. The contract is versioned if standard setting
changes the threshold or task envelope.

## Administration rules

- Listening recordings use Punjabi speakers and include documented regional
  breadth from B1 onward. Words-per-minute bands below guide form assembly; they
  are not a claim that Punjabi has one natural speaking rate.
- Pre-A1 and A1 directions may also be supplied in a declared support language,
  so misunderstanding directions is not confused with Punjabi proficiency.
  From A2 onward directions are in Punjabi, with one unscored example.
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

## Level envelopes

The later `task-shapes/<level>.json` inventories must make every part below
executable without expanding its input, response, timing, replay, interaction,
or aid boundary.

### pre-A1

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 10 | 6 Gurmukhi shape/word matches, 4 short phrase or notice matches, and 4 personal-detail selections; no text over 8 words |
| listening | 10 | 6 sound/word recognitions, 4 greeting-response choices, and 4 personal-detail selections; 70–90 wpm, two plays, 8-second inter-item pauses |
| writing | 12 | 2 delayed-copy items, 4 one-word dictation items, and 2 independently produced personal or greeting responses; no visible answer model during scoring |
| speaking | 8 | return a greeting, state a name or chosen identity, answer 3 familiar one-turn questions, and make 1 short request; the interlocutor may repeat once slowly |

Writing instruction proves observe/trace, guided copy, delayed copy, and
dictation/transcription in the learning record. Only delayed recall, dictation,
and independent production earn paper points; tracing and visible copying are
instructional supports, not pass evidence.

### A1

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 20 | 3 parts and 18–22 items across signs, forms, messages, and personal descriptions; 250–350 source words total, no text over 90 words |
| listening | 18 | 3 parts and 15–18 items across announcements, short exchanges, and a personal account; 90–110 wpm, two plays |
| writing | 20 | complete a practical form, then write a 30–40 word Gurmukhi message for a named reader and purpose |
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
| writing | 75 | synthesise two short sources in 180–220 words, then produce a 300–380 word audience-aware argument, report, or critical response |
| speaking | 22 | 5-minute source-based presentation, collaborative synthesis, and sustained challenge and defence; 15 minutes preparation with paper notes only |

### C2

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 90 | 4 parts and 34–42 items across stylistically varied specialist, public, and literary texts; 4,000–5,000 source words, including ambiguity and intertextual stance |
| listening | 60 | 4 parts and 28–34 items across rapid multiparty interaction, extended argument, narrative, and culturally dense media; natural 165–200 wpm variation, recordings played once |
| writing | 90 | synthesise and reframe multiple sources for one audience in 220–280 words, then produce a precise 420–520 word extended text in a contrasting genre or register |
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
