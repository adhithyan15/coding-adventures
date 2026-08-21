# Coding Adventures Latin Assessment

**Version:** 1.0 target contract, 2026-08-21

**Basis:** project-defined CEFR-aligned equivalent

**Status:** target specified; pre-A1 and A2–C2 task inventories, mocks,
calibration, and human validation are still backlog. The A1 task inventory exists,
but that alone is not readiness evidence.

This specification names the assessment that the complete Latin book must
eventually prepare a book-only learner to pass. It is not an external
qualification. The learner-facing name at each rung is **Coding Adventures
Latin <Level> Assessment — project-defined equivalent**.

The checked-in [`assessment.json`](assessment.json) is the machine-readable
contract. Its paths name required future artifacts. A path is a dependency, not
evidence that an inventory, rubric, answer key, or mock exists. All named assets,
two timed mock passes, and a book-only human pilot must land before a readiness
claim is permitted.

## Evidence and scope

The proficiency backbone is the Council of Europe's 2020 CEFR Companion Volume,
including pre-A1 descriptors and separate reception, production, interaction,
mediation, and written-production scales. CEFR can structure a transparent Latin
assessment, but the Council of Europe does not validate an examination's claimed
link to CEFR.

Euroclassica's European Curriculum for Latin is valuable Latin-specific context.
It publishes four levels — Vestibulum, Ianua, Palatium, and Thesaurus — with
competences in lexis, morphology, syntax, texts, and cultural background. Its
ELEX archive includes European Latin Exam papers. That framework is not a
pre-A1-to-C2 assessment with independently passed reading, listening, writing,
and speaking papers, so this project does not rename or imply equivalence to an
ELEX certificate. Every target here remains `project-defined`.

The assessed language is Classical Latin, with post-classical and Neo-Latin
texts introduced where a task labels them. The default pronunciation model is
the reconstructed Classical model taught by the book. A documented consistent
alternative, including Ecclesiastical pronunciation, receives full credit when
it preserves the contrasts required by the task and remains intelligible. No
candidate is penalised for the absence of one imagined native accent.

Macrons are reading and pronunciation aids, not ordinary mandatory spelling.
They are scored only when a prompt explicitly tests vowel quantity or supplies
macrons for read-aloud performance. `u/v` and `i/j` conventions receive credit
when used consistently. Editorially supplied punctuation is not treated as an
ancient authorial feature.

Sources checked 2026-08-21:

- [CEFR Companion Volume and language versions](https://www.coe.int/en/web/common-european-framework-reference-languages/cefr-companion-volume-and-its-language-versions)
- [Council of Europe CEFR descriptors](https://www.coe.int/en/web/common-european-framework-reference-languages/cefr-descriptors)
- [Council of Europe framework and certification caveat](https://www.coe.int/en/web/common-european-framework-reference-languages/introduction-and-context)
- [Euroclassica European Curriculum for Latin](https://latein.schule.at/portale/latein/didaktik-lehrplan/lehrplan/detail/european-curriculum-for-latin.html)
- [Euroclassica ELEX archive](https://www.euroclassica.eu/portale/euroclassica/eccl/elex.html)

## Pass rule

Reading, listening, writing, and speaking are separate 100-point papers. A
candidate must score at least **60/100 on every paper in the same
administration**. There is no aggregate compensation: a strong translation or
reading score cannot hide absent listening, writing, or speaking. An unattempted
required part scores zero. A complete pass also requires both published timed
mocks at the rung to have been passed under the same independent thresholds.

Writing and speaking use four equally weighted analytic dimensions:

1. task fulfilment and relevant content;
2. comprehensibility, organisation, and interaction;
3. range and control of Latin vocabulary, morphology, and syntax;
4. orthographic consistency for writing, or pronunciation and fluency for
   speaking.

Each dimension is scored 0–5, then scaled to the paper's points. A response that
is not meaningfully in Latin cannot earn task-fulfilment credit. At pre-A1 and
A1, an inflectional or quantity error loses language-control credit only to the
extent appropriate to the task; it does not erase otherwise communicated
content. From A2 upward, repeated uncontrolled morphology affects control while
meaning and organisation remain separately scored.

The 60% threshold is a transparent provisional project standard, not validation
evidence. Before release, at least eight qualified Latin educators, historical
linguists, or active-Latin assessment specialists must perform a modified Angoff
review. Two raters must double-score at least 50 writing and 50 speaking samples
per rung. A third rater adjudicates scaled totals that differ by more than 10
points or disagreements about a documented orthographic or pronunciation
convention. The contract is versioned if standard setting changes a threshold or
task envelope.

## Administration rules

- Listening forms use trained Latin readers and identify the pronunciation model
  in form metadata. Words-per-minute bands are form-assembly targets, not claims
  about one authentic ancient speech rate.
- Pre-A1 and A1 directions may also appear in a declared support language. From
  A2 onward, directions are in clear Latin with one unscored example.
- No dictionary, grammar, commentary, translation, spell-checker, or generative
  assistant is allowed unless a later task inventory explicitly publishes a
  bounded reference aid. Unseen source glosses supplied by the form are allowed.
- Listening replay rules are fixed by each inventory. A technical restart
  replaces an affected recording; it is not an extra candidate-selected replay.
- Speaking is recorded and independently scored by two trained raters. A
  cooperative interlocutor follows a memorised protocol and may not translate.
- Full mocks use unseen prompts and target timing. Five-minute lessons prepare
  each component gently; full assessments retain continuous timing because they
  are evidence, not instruction.

## Level envelopes

Each later `task-shapes/<level>.json` inventory must make the relevant envelope
executable without expanding input, response, timing, replay, interaction, or
aid boundaries.

### pre-A1

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 15 | 8 familiar word/label matches and 6 memorised phrase or micro-dialogue matches; no text over 8 words |
| listening | 10 | 8 familiar sounds, words, and greeting responses at 45–65 wpm; two plays and 8-second inter-item pauses |
| writing | 10 | 2 delayed-recall words, 3 one- to three-word dictations, and 2 independently produced labels or greeting responses; no visible answer model during scoring |
| speaking | 8 | return a greeting, state a name or chosen identity, answer 3 familiar one-turn questions, and make 1 short request; one slow repetition is permitted |

Writing instruction proves observe/trace, guided copy, delayed copy, and
dictation/transcription. Only delayed recall, dictation, and independent
production earn points; tracing and visible copying remain instructional.

### A1

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 35 | the committed A1 inventory: labels/inscriptions, one social exchange, and one 120–160 word adapted account; 25 scored items |
| listening | 25 | sound contrasts, four mini-exchanges, and one 100–140 word supported dialogue at 55–85 wpm; 25 scored items and two plays |
| writing | 30 | five short dictations, four original controlled sentences, and a 35–50 word note or account for a named reader |
| speaking | 12 | a 50–70 word read-aloud, a 60–90 second personal presentation, and a 3–4 minute cooperative exchange; 10 minutes preparation |

This envelope is already encoded in `task-shapes/a1.json`. It is an inventory,
not proof that the current book teaches or that a learner passes every part.

### A2

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 45 | 3 parts across adapted narrative, correspondence, and inscriptions or material culture; 650–850 source words total, including one short evidence citation |
| listening | 30 | announcements, exchanges, and one 2–3 minute connected account at 65–95 wpm; short items twice and the long item twice |
| writing | 45 | a 30–45 word functional response and a 90–120 word connected description, narration, or message |
| speaking | 15 | interview, 2-minute prepared account, read-aloud with phrasing, and a cooperative information-exchange task; 10 minutes preparation |

### B1

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 60 | 4 parts across adapted prose, verse, inscription, and historical or cultural commentary; 1,300–1,700 source words with explicit inference and evidence selection |
| listening | 40 | transactions, narrative, interview, and a 4–5 minute talk at 75–105 wpm; short items twice and long items once after preview |
| writing | 50 | a 60–80 word interaction text and a 160–210 word connected narrative, description, or explanation for a named audience |
| speaking | 18 | interview, 3-minute account, collaborative planning, sight-supported reading, and follow-up discussion; 10 minutes preparation |

### B2

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 75 | 4 parts across lightly adapted and unadapted prose, verse, epigraphy, and scholarly framing; 2,200–2,800 source words, including rhetoric and implicit stance |
| listening | 50 | multiple readers, extended narrative or argument, dialogue, and verse at 85–115 wpm; long recordings played once after an orienting preview |
| writing | 60 | a 100–130 word interaction or mediation text and a 250–320 word narrative, report, or reasoned argument in the required register |
| speaking | 20 | 4-minute presentation, collaborative problem-solving, interpretation of a short source, and defended discussion; 12 minutes preparation |

### C1

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 90 | 4 parts across unadapted prose and verse, inscriptions or manuscripts, and modern scholarship; 3,500–4,500 source words with textual ambiguity and intertextual stance |
| listening | 60 | unscripted active-Latin discussion, lecture, literary reading, and narrative at 90–125 wpm with natural variation; recordings played once |
| writing | 75 | synthesise two Latin sources in 200–250 words, then produce a 330–420 word audience-aware argument, narration, or critical response |
| speaking | 25 | 5-minute source-based presentation, collaborative synthesis, literary interpretation, and sustained challenge and defence; 15 minutes preparation with Latin notes only |

### C2

| paper | minutes | required task envelope |
|---|---:|---|
| reading | 105 | 4 parts across stylistically varied classical, late, medieval, and Neo-Latin sources plus textual scholarship; 5,000–6,000 source words, including ambiguity, register, and transmission problems |
| listening | 70 | rapid multiparty active-Latin interaction, extended argument, literary performance, and culturally dense lecture or media at 95–135 wpm; recordings played once |
| writing | 90 | synthesise and reframe multiple sources for one audience in 250–320 words, then produce a precise 500–650 word extended text in a contrasting genre or register |
| speaking | 30 | 6-minute synthesis or mediation presentation, multiparty negotiation, close interpretation, and sustained defence requiring reformulation for a second audience; 15 minutes preparation with Latin notes only |

## Required artifacts and readiness language

For each rung, implementation must add:

1. the four-skill JSON task inventory named by `assessment.json`;
2. two complete timed mock forms with unseen input and prompts;
3. a shared analytic rubric plus task-specific scoring notes;
4. one answer key per mock, including documented pronunciation and orthographic
   alternatives;
5. rater training samples, double-scoring evidence, and adjudication records;
6. the pre-registered book-only human-validation report.

Until items 1–4 exist, reports may say **target contracted** only. After a
learner passes both mocks they may say **mock ready**. Only the pre-registered
human study can support **learner proven**, and its result must never be inferred
from level labels, corpus coverage, an ELEX paper, or the A1 task inventory alone.
