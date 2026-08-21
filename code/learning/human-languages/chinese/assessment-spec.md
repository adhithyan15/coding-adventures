# Coding Adventures Chinese Assessment

**Version:** 1.0 transitional target contract, 2026-08-21

**Basis:** project-defined CEFR ladder aligned to GF0025-2021 and the changing
HSK 3.0 examination system

**Status:** target specified; task inventories, mocks, calibration, and human
validation remain backlog

This specification names the assessment that the Mandarin Chinese book must
eventually prepare a book-only learner to pass. It is deliberately dated because
the external system is in transition. HSK Levels 7–9 already operate as one
advanced five-skill exam, while HSK 3.0 Levels 1–6 are still being globally
trialled in 2026.

The checked-in [assessment.json](assessment.json) is the machine-readable
contract. Its future paths are requirements, not evidence that the corresponding
task inventory, rubric, answer key, mock, or validation study exists.

## What is official, and what is not

China's Ministry of Education standard GF0025-2021 defines three broad stages
and nine levels. It describes listening, speaking, reading, writing, and
translation, together with syllable, character, vocabulary, and grammar
benchmarks. It is an official Chinese-proficiency standard.

The official Chinese Test Service describes HSK 3.0 as assessing listening,
speaking, reading, writing, and—at advanced levels—translation. Its September
2026 trial requires candidates for Levels 3–6 to register for the corresponding
speaking paper. The same notice lists no paired speaking paper for Levels 1–2.
The operational combined Levels 7–9 exam has 98 items over roughly 210 minutes
and reports listening, reading, writing, translation, and speaking scores.

None of those official sources publishes the CEFR correspondence used by this
curriculum. The following is therefore project judgement:

| curriculum rung | project external anchor |
|---|---|
| A1 | HSK 3.0 Level 1-aligned |
| A2 | HSK 3.0 Level 2-aligned |
| B1 | HSK 3.0 Level 3-aligned |
| B2 | HSK 3.0 Level 4-aligned |
| C1 | HSK 3.0 Level 6-aligned |
| C2 | HSK Level 9-aligned |

Level 5 remains a substantial milestone on the road to the C1 target; Levels 7
and 8 remain substantial milestones on the road to C2. The table does not say
that an HSK certificate is a CEFR certificate. Every target is labelled
project-defined, and every future report must preserve that label.

Sources checked 2026-08-21:

- [Ministry of Education announcement for GF0025-2021](https://www.moe.gov.cn/jyb_xwfb/gzdt_gzdt/s5987/202103/t20210329_523304.html)
- [Ministry of Education explanation of the nine-level standard and HSK transition](https://www.moe.gov.cn/jyb_xwfb/s271/202104/t20210402_524194.html)
- [Official September 2026 HSK 3.0 trial notice](https://www.chinesetest.cn/notice)
- [Official HSK Levels 7–9 structure](https://www.chinesetest.cn/HSK/7-9)

## Four-skill pass rule

Reading, listening, writing, and speaking are separate 100-point curriculum
papers. A candidate must score at least 60/100 on every paper in the same
administration. A high reading result cannot compensate for failed writing.
Where the named HSK form has an official pass or level-award rule, the candidate
must satisfy that external rule as well. The local 60% floor never overwrites an
official threshold or IRT-based level decision.

Translation or mediation remains mandatory when the external HSK target
requires it. In this four-skill schema, written translation is scored inside the
writing paper and oral translation or reformulation inside speaking. It cannot
be omitted merely because there is no fifth top-level skill key.

Writing and speaking use four equally weighted analytic dimensions:

1. task fulfilment and relevant content;
2. comprehensibility, organisation, and interaction;
3. vocabulary, grammar, and register control;
4. character and punctuation control for writing, or tones, pronunciation, and
   fluency for speaking.

Pinyin is accepted only when a task explicitly licenses it. A character task
must state whether handwriting, typed input, or either mode is required. Script
support that is visible during tracing or copying is learning evidence, not
independent pass evidence.

Before a “learner proven” claim, both timed mocks at the rung must be passed and
a preregistered book-only pilot must show every skill passing independently.
Two trained raters double-score writing and speaking; a third adjudicates
material disagreement. External-format mocks require a Chinese assessment
specialist and documented calibration against official sample material or
candidates with recent official results.

## Administration and transition rules

- Lessons remain five minutes or shorter. Partial mocks build endurance gently;
  full mocks keep the external target's continuous timing.
- Standard Mandarin and simplified characters are the initial assessed variety.
  Tasks state when a documented regional pronunciation or traditional-character
  response is acceptable.
- Pre-A1 and A1 directions may include a declared support language. From A2,
  Chinese directions include one unscored example.
- No dictionary, translator, handwriting recogniser, pinyin input aid,
  spell-checker, or generative assistant is allowed unless the task or a recorded
  accessibility accommodation explicitly permits it.
- Project mocks use original items; they do not copy protected live HSK items.
- Every annual target review checks whether Levels 1–6 have left trial status,
  whether their speaking/writing sections or score rules changed, and whether an
  official international-framework alignment has been published.

## Gentle writing ladder

Writing begins with one visible contour or stroke path and advances only after
evidence at each step:

1. observe and trace a visible pinyin contour or character component;
2. guided-copy one character with formation cues;
3. look, hide, write, compare, and repair one known character or word;
4. transcribe a heard syllable or pinyin cue into the required character;
5. choose and order known material in controlled composition;
6. write connected text for a named reader and purpose;
7. produce under the target paper's time and rubric.

A learner is never asked to memorise a page of characters before writing one,
and recognising a character does not count as producing it.

## Level envelopes

### pre-A1

This project-defined HSK precursor tests a tiny greeting exchange, yes/no,
identity, and a few classroom cues. Reading and listening inputs are isolated
known words or one short turn. Writing samples one delayed character or known
word, short dictation/transcription, and one bounded independent response.
Speaking covers a greeting, a name, one yes/no response, and one memorised
request. No response is longer than one short turn.

### A1

External anchor: the current HSK 3.0 Level 1 construct. Because the 2026 trial
notice does not pair Level 1 with a speaking paper, the complete target adds
independently scored speaking and productive writing. The candidate handles
high-frequency personal and survival exchanges, reads short signs/messages,
writes a small form and short message, and completes a rehearsed role-play.

### A2

External anchor: HSK 3.0 Level 2. The complete target adds or separates any
productive skill the live form does not independently score. The candidate
handles routine transactions and personal accounts, writes a short functional
response plus a connected message, and sustains a simple information-exchange
role-play.

### B1

External anchor: HSK 3.0 Level 3 with its corresponding speaking component under
the 2026 trial rules. The complete target requires independently scored
reception and production even if the external result aggregates them. Writing
includes functional correspondence and a connected narrative or opinion;
speaking includes an account, collaborative planning, and follow-up discussion.

### B2

External anchor: HSK 3.0 Level 4 with its corresponding speaking component.
The candidate reads and listens across reporting, explanation, and argument,
writes a structured audience-aware report or position, and sustains a
presentation plus defended discussion with register control.

### C1

External anchor: HSK 3.0 Level 6, with Level 5 retained as an intermediate
milestone rather than silently skipped. The complete target requires dense
public, professional, academic, and literary reception; multi-source written
synthesis; an extended argument; source-based presentation; and sustained
interaction. Translation or mediation tasks are introduced before the advanced
external target demands them.

### C2

External anchor: an HSK Levels 7–9 result at Level 9. The complete target
preserves the live exam's listening, reading, writing, translation, and speaking
construct, then reports the four curriculum skills independently. Tasks sample
unfamiliar specialist, public, academic, and literary domains; require precise
multi-source writing in contrasting genres; and include oral and written
translation, reformulation, negotiation, and sustained defence.

## Required artifacts and readiness language

Each rung still needs:

1. complete four-skill task-shape inventories with dated external provenance;
2. two original full timed mock forms;
3. analytic rubrics and task-specific scoring notes;
4. answer keys, acceptable variants, and rater-training samples;
5. calibration and double-scoring evidence;
6. a preregistered book-only human-validation report.

Until items 1–4 exist, reports may say **target contracted** only. Passing both
calibrated mocks supports **mock ready**. Only the human study supports
**learner proven**. HSK alignment, chapter count, vocabulary count, or a level
label alone supports none of those claims.
