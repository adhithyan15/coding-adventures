# Coding Adventures Persian Assessment

**Version:** 1.0 target contract, 2026-08-21

**Basis:** project-defined CEFR-aligned ladder, followed by the external SAMFA
Academic capstone

**Status:** targets specified; task inventories, mocks, calibration, capstone
forms, and human validation are still backlog

This specification names the assessments that the complete Persian book must
prepare a book-only learner to pass. The seven curriculum rungs are transparent
project-defined equivalents. After C2, the learner must also be prepared for the
current **SAMFA Academic** provider exam. SAMFA is not called C2: the provider
source used here publishes no CEFR mapping.

The checked-in [`assessment.json`](assessment.json) is the machine-readable
contract. Its paths name required future artifacts. A path is a dependency, not
evidence that an inventory, rubric, answer key, or mock exists. All named assets,
two timed mock passes at every rung and for SAMFA, and a book-only human pilot
must land before a readiness claim is permitted.

## Evidence and target choice

The curriculum ladder uses the Council of Europe's 2020 CEFR Companion Volume,
including pre-A1 descriptors and separate reception, production, interaction,
mediation, and written-production scales. CEFR supports transparent assessment
design but does not accredit this project or validate a claimed exam mapping.

SAMFA Academic is a real external four-skill destination. The provider notice
for its eleventh administration publishes four 60-point skills, 30 listening
questions in 60 minutes, 30 reading questions in 60 minutes, two written essays
in 60 minutes, and a 15-minute oral exam. It states that there is no universal
pass/fail result, while Iranian universities currently require 50% in each of
the four skills. Listening is common across academic fields; reading, writing,
and speaking prompts vary by engineering, social sciences, humanities, or
medical sciences.

That source does not publish a defensible mapping from SAMFA to A1–C2. It also
does not expose enough prompt, response-length, rubric, and sample-form detail to
replace the project's seven-rung learning ladder. The contract therefore keeps
SAMFA as `cefrRelation: not-mapped`, attempted after C2, with its own future task
inventory and mocks. “After C2” is a dependency boundary, not an equivalence.

Sources checked 2026-08-21:

- [CEFR Companion Volume and language versions](https://www.coe.int/en/web/common-european-framework-reference-languages/cefr-companion-volume-and-its-language-versions)
- [Council of Europe CEFR descriptors](https://www.coe.int/en/web/common-european-framework-reference-languages/cefr-descriptors)
- [Council of Europe framework and certification caveat](https://www.coe.int/en/web/common-european-framework-reference-languages/introduction-and-context)
- [SAMFA eleventh-administration provider notice](https://fa.irancultura.it/11a-edizione-esame-samfa-bando-ufficiale/)

## Language, script, and variety policy

The project-defined ladder assesses contemporary standard Iranian Persian in
the Persian script. Reading and writing move from ordinary informal messages to
formal public, literary, and academic registers. Listening and speaking include
the relationship between formal written-style Persian and common educated
colloquial Iranian speech; a learner must not reach an academic capstone having
heard only careful book pronunciation.

Early writing isolates right-to-left direction, joining behaviour, dot patterns,
letter families, spacing, and the zero-width non-joiner before words are combined.
Short vowels may be supplied as temporary instructional aids but are not required
in ordinary independent spelling. Iranian yeh and kaf are the output model;
Unicode-equivalent Arabic code points are normalised before scoring rather than
treated as language errors. ZWNJ and spacing variants receive credit when they
remain readable and internally consistent, with stricter control introduced
gradually for formal writing.

Documented Dari vocabulary, morphology, and pronunciation receive credit in the
project-defined ladder when used consistently and when the task is not explicitly
about Iranian usage. Tajik Cyrillic is outside this track's present script
contract. SAMFA responses are governed by the provider's rules; this project
does not promise that every variety accepted in its own ladder will receive the
same treatment from an external rater.

## Pass rule

For each project-defined rung, reading, listening, writing, and speaking are
separate 100-point papers. A candidate must score at least **60/100 on every
paper in the same administration**. There is no aggregate compensation. An
unattempted required part scores zero. A complete pass also requires both timed
mocks at that rung under the same thresholds.

Writing and speaking use four equally weighted analytic dimensions:

1. task fulfilment and relevant content;
2. comprehensibility, organisation, and interaction;
3. range and control of Persian vocabulary and grammar;
4. Persian orthographic control for writing, or pronunciation and fluency for
   speaking.

Each dimension is scored 0–5, then scaled to the paper's points. Register choice
is scored only after the task has taught and requested a contrast. At lower
rungs, colloquial contractions do not erase an otherwise successful spoken task;
at higher rungs, the candidate must control formal and colloquial choices for
audience and purpose.

For SAMFA Academic, the provider's 0–60 score in each skill remains authoritative.
The current external threshold recorded here is **50% in every skill**, not an
aggregate 120/240 shortcut. If a chosen university or later provider edition
requires more, the higher current threshold controls and the contract is
versioned.

Before release, at least eight qualified Persian educators, linguists, or
assessment specialists must perform a modified Angoff review of every
project-defined rung. Two raters must double-score at least 50 writing and 50
speaking samples per rung. SAMFA mock rubrics must be reviewed by educators with
current provider-form experience. A third rater adjudicates scaled totals that
differ by more than 10 points or disagreements about a documented variety.

## Administration rules

- Listening forms use Persian speakers and declare region, register, and target
  speed. Speed bands guide form assembly; they do not define one natural rate.
- Pre-A1 and A1 directions may also appear in a declared support language. From
  A2 onward, directions are in Persian with one unscored example.
- No dictionary, grammar reference, spell-checker, translator, or generative
  assistant is allowed unless a later inventory explicitly publishes a bounded
  aid. Accessibility accommodations may change mode, not construct.
- Listening replay rules are fixed by each inventory. A technical restart
  replaces an affected recording and is not an extra candidate-selected replay.
- Speaking is recorded and independently scored by two trained raters. A
  cooperative interlocutor follows a memorised protocol.
- Full mocks use unseen prompts and target timing. Five-minute lessons prepare
  components gently; complete assessments retain continuous timing because they
  are evidence, not instruction.

## Project-defined level envelopes

Each later `task-shapes/<level>.json` inventory must make the relevant envelope
executable without expanding input, response, timing, replay, interaction, or
aid boundaries.

### pre-A1

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 10 | 6 taught letter/word matches, 4 memorised phrase or sign matches, and 4 personal-detail selections; no text over 8 words |
| listening | 10 | 6 sound/word recognitions, 4 greeting responses, and 4 personal details at 70–90 wpm; two plays and 8-second pauses |
| writing | 12 | 2 delayed-recall items, 4 one-word dictations, and 2 independently produced personal or greeting responses; no visible answer model during scoring |
| speaking | 8 | return a greeting, state a name or chosen identity, answer 3 familiar one-turn questions, and make 1 short request; one slow repetition is permitted |

Writing instruction proves observe/trace, guided copy, delayed copy, and
dictation/transcription. Only delayed recall, dictation, and independent
production earn points; tracing and visible copying remain instructional.

### A1

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 20 | 3 parts and 18–22 items across signs, forms, messages, and personal descriptions; 250–350 source words total |
| listening | 18 | 3 parts and 15–18 items across announcements, short exchanges, and a personal account at 90–110 wpm; two plays |
| writing | 20 | complete a practical form, then write a 30–40 word Persian message for a named reader and purpose |
| speaking | 10 | personal interview, 1-minute prepared description, and a simple transactional role-play; 5 minutes preparation |

### A2

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 30 | 3 parts and 22–26 items across notices, correspondence, and connected everyday texts; 550–750 source words total |
| listening | 25 | 3 parts and 18–22 items across announcements, conversations, and a 2–3 minute account at 110–130 wpm; two plays |
| writing | 30 | a 25–35 word functional response and a 70–90 word connected message or description |
| speaking | 12 | interview, 2-minute picture or topic account, and an information-exchange role-play; 5 minutes preparation |

### B1

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 45 | 4 parts and 26–32 items across correspondence, public information, narrative, and an informational article; 1,100–1,400 source words |
| listening | 35 | 4 parts and 22–28 items across transactions, interviews, narrative, and a short talk at 130–150 wpm; long items played once after preview |
| writing | 45 | a 50–70 word functional text and a 130–170 word connected narrative, description, or opinion for a named audience |
| speaking | 15 | interview, 3-minute prepared account, collaborative planning, and follow-up discussion; 10 minutes preparation |

### B2

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 60 | 4 parts and 30–36 items across argument, reporting, instructions, and literary or cultural prose; 1,800–2,300 source words |
| listening | 45 | 4 parts and 24–30 items using formal and colloquial speakers at 150–170 wpm; long recordings played once |
| writing | 60 | a 100–130 word interaction text and a 220–280 word report, article, narrative, or reasoned argument in the required register |
| speaking | 18 | 4-minute presentation, collaborative problem-solving, and defended discussion with follow-up questions; 10 minutes preparation |

### C1

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 75 | 4 parts and 32–40 items across dense public, professional, academic, and literary texts; 2,800–3,500 source words, including implicit stance |
| listening | 50 | 4 parts and 26–32 items across unscripted discussion, lecture, interview, and narrative at 160–185 wpm; recordings played once |
| writing | 75 | synthesise two short sources in 180–220 words, then produce a 300–380 word audience-aware argument, report, or critical response |
| speaking | 22 | 5-minute source-based presentation, collaborative synthesis, and sustained challenge and defence; 15 minutes preparation with Persian notes only |

### C2

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 90 | 4 parts and 34–42 items across stylistically varied specialist, public, literary, and historical texts; 4,000–5,000 source words |
| listening | 60 | 4 parts and 28–34 items across rapid multiparty interaction, extended argument, narrative, and culturally dense media at natural 165–200 wpm variation |
| writing | 90 | synthesise and reframe multiple sources in 220–280 words, then produce a precise 420–520 word extended text in a contrasting genre or register |
| speaking | 25 | 6-minute synthesis or mediation presentation, multiparty negotiation, and sustained defence requiring reformulation for a second audience; 15 minutes preparation |

## External capstone: SAMFA Academic

The future `capstones/samfa-academic.json` must preserve the provider envelope
published for the eleventh administration:

| skill | provider envelope | threshold recorded by this contract |
|---|---|---:|
| listening | 30 questions, 60 minutes, common across academic fields | 30/60 |
| reading | 30 questions, 60 minutes, field-specific | 30/60 |
| writing | 2 essays, 60 minutes, field-specific | 30/60 |
| speaking | oral examination, 15 minutes, field-specific | 30/60 |

The inventory must keep provider-unknown prompt lengths, replay counts, scoring
criteria, and aid rules explicitly unknown until stable primary material is
available. It must not fill those gaps by analogy to IELTS, TOEFL, or the
project-defined C2 paper. If SAMFA changes, the provider edition and task shapes
are versioned before new readiness claims.

## Required artifacts and readiness language

For each project-defined rung and for SAMFA, implementation must add:

1. the four-skill task inventory named by `assessment.json`;
2. two complete timed mock forms with unseen input and prompts;
3. a shared analytic rubric plus task-specific scoring notes;
4. one answer key per mock, including documented orthographic and variety notes;
5. rater training samples, double-scoring evidence, and adjudication records;
6. the pre-registered book-only human-validation report.

Until items 1–4 exist, reports may say **target contracted** only. Passing two
project mocks can support **mock ready** for that target. SAMFA readiness requires
passing both SAMFA-shaped mocks at the current provider thresholds. Only the
pre-registered human study—and, for the external capstone, actual provider
results—can support **learner proven**.
