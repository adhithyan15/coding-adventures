# Coding Adventures Italian Assessment

**Version:** 1.1 target contract, 2026-09-13

**Basis:** CILS from A1 upward; a project-defined precursor below it

**Status:** target specified; the pre-A1 task inventory is checked in, while
A1-C2 inventories, mocks, calibration, and human validation remain backlog

This specification names the assessment that the complete Italian book must
eventually prepare a book-only learner to pass. At pre-A1 the learner-facing
name is **Coding Adventures Italian pre-A1 Assessment — project-defined CILS A1
precursor**. It does not claim that the current book is exam-ready.

No `assessment.json` is checked in for Italian yet, and that omission is
deliberate. The machine-readable contract names required artifacts by path, and
the artifact gate treats a path that leads nowhere as an error rather than as a
promise. The contract is therefore written after the inventories, mocks, rubrics
and answer keys it points at, not before them.

## Evidence and scope

The proficiency backbone is the Council of Europe's 2020 CEFR Companion Volume,
including its pre-A1 descriptors. CEFR is a framework for describing proficiency
and developing transparent examinations, and the Council of Europe states that
it does not verify or validate an examination's claimed link to CEFR.

Italian is the first track in this corpus whose awarding body does not need
approximating. **CILS** — Certificazione di Italiano come Lingua Straniera,
issued by the Università per Stranieri di Siena — certifies six levels, A1
through C2, aligned to the CEFR reference levels. From A1 upward, CILS is the
target. It is named, it is real, and this book aims at it rather than at a
project invention.

### What CILS does that the four-skill model does not

A CILS examination has **five** parts, not four:

1. ascolto (listening);
2. lettura (reading);
3. produzione scritta (written production);
4. produzione orale (oral production);
5. **analisi delle strutture di comunicazione** — an analysis of the structures
   of communication in Italian.

The fifth has no slot in a four-skill contract, and pretending it does not exist
would leave a learner surprised on the day. It enters this ladder at **A1**,
where the CILS target itself begins. There is no CILS pre-A1, so the pre-A1 rung
below it is a project-defined precursor with four papers and no structures
paper: at that level there is not yet enough structure to analyse.

### The pass rule, which this project does not have to argue with

Every other track in this corpus states its independent-skill rule against the
real exam's grain. Italian does not, because CILS already refuses compensation.

- At A1 and A2 each part is scored out of **12**, and a candidate must reach
  **7**. Fall below 7 on a single part and the examination is not passed,
  whatever the other parts scored.
- From B1 to C2 each part is scored out of **20**, and a candidate must reach
  **11**; a failed part can be retaken, or capitalised, within eighteen months.

That is the same claim this project makes everywhere — four skills that move
together, and no strong reader carrying a silent writer — arrived at
independently by an awarding body. The 60% used below sits a little above the
7/12 CILS asks at A1 and matches the 11/20 it asks above; it is stated as a
project threshold and not as a CILS equivalence.

The assessed variety is contemporary standard Italian. The current track's
taught forms define the initial model. Documented regional forms receive credit
when used consistently and when they satisfy the task; a trained Italian
reviewer adjudicates unfamiliar forms.

Sources checked 2026-09-13:

- [CEFR Companion Volume and language versions](https://www.coe.int/en/web/common-european-framework-reference-languages/cefr-companion-volume-and-its-language-versions)
- [Council of Europe guidance on tests and examinations](https://www.coe.int/en/web/common-european-framework-reference-languages/tests-and-examinations)
- [Council of Europe introduction and certification caveat](https://www.coe.int/en/web/common-european-framework-reference-languages/introduction-and-context)
- [I livelli CILS — Università per Stranieri di Siena](https://cils.unistrasi.it/1/79/82/I_livelli_CILS.htm)
- [CILS structure and scoring — Centro Linguistico, Università di Pavia](https://cla.unipv.it/?page_id=54510)
- [Descrizione degli esami CILS — Istituto Italiano di Cultura di Buenos Aires](https://iicbuenosaires.esteri.it/it/lingua-e-cultura/certificazioni/gli-esami-di-certificazione/)

## Pass rule

Reading, listening, writing, and speaking are separate 25-point papers at
pre-A1. A candidate must score at least **60% on every paper in the same
administration**. There is no aggregate compensation. An unattempted required
part scores zero. From A1 the CILS structure and scoring above govern.

Writing and speaking use four equally weighted analytic dimensions:

1. task fulfilment and relevant content;
2. comprehensibility, organisation, and interaction;
3. range and control of Italian vocabulary and grammar;
4. Italian orthographic control for writing, or pronunciation and fluency for
   speaking.

Italian spelling is close to phonemic, which makes dimension 4 short and
specific rather than broad. Two things carry almost all of it:

- **double-consonant length.** *nono* and *nonno*, *casa* and *cassa*, *pena*
  and *penna* differ in nothing but how long one consonant is held. A learner
  who does not hear length does not write it, and the error changes the word
  rather than the spelling. This is the single most common orthographic error in
  Italian dictation and it is scored by name in every writing part;
- **the graphic accent on a final stressed vowel** — *è*, *perché*, *città*,
  *più*. Where Italian marks stress it marks it only there, and omitting it is
  an error even though nothing audible changes for a reader who already knows
  the word.

## The writing ramp, measured rather than assumed

Version 1.0 of this document recorded a gap: Italian had **one** writing lesson,
carrying observe-trace and guided-copy and nothing beyond, while the pre-A1
paper below requires a delayed-recall item and a dictation item. A learner the
book never took past guided copy cannot answer either.

**That gap is now paid.** Chapter 1 carries the full pre-A1 ladder on the one
word it already had:

| stage | lesson | what the hand is asked to do |
|---|---|---|
| observe-trace | `IT-C01-ciao` | notice the shape, trace it with the model visible |
| guided-copy | `IT-W01-ciao-guided-copy` | copy it, model still on the page |
| delayed-copy | `IT-W01-ciao-delayed-copy` | model covered, ten seconds, write from memory |
| dictation-transcription | `IT-W01-ciao-dictation` | nothing on the page — write from the sound |

Four stages on four letters, which is the point: the stages measure what the
hand is being asked to do, not how much language is on the page. The order is
load-bearing and enforced — a delayed copy is not valid evidence unless the
tracing and the guided copy come earlier in sequence.

**ciao** turns out to be the right word to do this on. Three sounds, four
letters: the **i** is written for the **c** to read, not for the learner to say.
So the delayed copy's expected miss is the **i** (a hand writing from memory
drops the letter it never heard), and the dictation is where that stops being a
fact about one word and becomes a habit — *ch* before *a*, *o* or *u* puts an
**i** on the page, and nothing in the sound will remind you.

## Administration rules

- Listening recordings use Italian speakers. Words-per-minute bands below are
  form-assembly targets, not a claim that Italian has one natural speaking rate.
- Pre-A1 directions may also be supplied in a declared support language, so
  misunderstanding directions is not confused with Italian proficiency. From A1
  onward directions are in Italian, with one unscored example.
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

The 60% pre-A1 threshold is a transparent provisional project standard, not
validation evidence. Before release, at least eight qualified Italian educators
or linguists must perform a modified Angoff review; two raters must double-score
at least 50 writing and 50 speaking samples per rung; and a book-only pilot must
report every skill separately.

## Level envelopes

The later `task-shapes/<level>.json` inventories must make every part below
executable without expanding its input, response, timing, replay, interaction,
or aid boundary. From A1 the envelope must additionally accommodate the CILS
structures paper. The A1-C2 bands are the project's own shared ladder, identical
across tracks by design.

### pre-A1

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 8 | known-word matches and one choose-the-reply part; no text over 8 words |
| listening | 7 | greeting recognition and catching a spoken name; two plays |
| writing | 10 | 1 delayed-recall item, 1 dictation item, and 1 bounded independent response; no visible answer model during scoring |
| speaking | 5 | return a greeting, state a name or chosen identity, answer familiar one-turn questions, and make 1 short request; 2 minutes preparation |

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
