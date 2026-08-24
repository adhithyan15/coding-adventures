# Coding Adventures Japanese Assessment

**Version:** 1.0 target contract, 2026-08-20

**Basis:** project-defined four-skill assessment with official JLPT receptive
anchors and JF Standard/CEFR-aligned production companions

**Status:** target specified; task inventories, mocks, calibration, and human
validation remain backlog

This specification names the assessment that the Japanese book must eventually
prepare a book-only learner to pass. It deliberately does not call the complete
assessment “the JLPT.” JLPT supplies real language-knowledge, reading, and
listening anchors from A1 through C1, but it does not assess speaking, writing,
or interaction. The complete target is therefore a project-defined hybrid:

1. an official-format JLPT receptive component wherever an official CEFR
   reference band exists; and
2. independently scored writing and speaking papers derived from JF Standard
   and CEFR Can-do descriptors.

Pre-A1 is the gentle runway below the external ladder. C2 is beyond the official
JLPT CEFR reference range. Both are wholly project-defined.

The checked-in [assessment.json](assessment.json) is the machine-readable
contract. Its paths name required future artifacts; they are dependencies, not
evidence that an inventory, rubric, answer key, or mock already exists.

## Official boundary

From the December 2025 JLPT, score reports may show a CEFR reference level for
candidates who pass the level they sat. The published correspondences are:

| CEFR reference | official JLPT result |
|---|---|
| A1 | pass N5 with total score 80 or above |
| A2 | pass N4 with 90 or above, or pass N3 with 95–103 |
| B1 | pass N3 with 104 or above, or pass N2 with 90–111 |
| B2 | pass N2 with 112 or above, or pass N1 with 100–141 |
| C1 | pass N1 with 142 or above |

Those are scaled total scores, not raw percentages. A candidate must also clear
every official sectional pass mark: 38/120 for combined language knowledge and
reading plus 19/60 for listening at N4–N5; 19/60 in each of language knowledge,
reading, and listening at N1–N3. An internal mock must not print an “official
JLPT score” unless a documented calibration study supports the conversion.

The official CEFR indication covers only the competence JLPT tests: language
knowledge and reception. It excludes production and interaction. Passing JLPT
alone can therefore satisfy only the receptive side of this contract.

Sources checked 2026-08-20:

- [JLPT CEFR reference indication](https://www.jlpt.jp/e/about/cefr_reference.html)
- [JLPT scoring sections and pass rules](https://www.jlpt.jp/e/guideline/results.html)
- [JF Standard overview](https://www.jfstandard.jpf.go.jp/summaryen/ja/render.do)
- [JF Standard Can-do resources](https://www.jfstandard.jpf.go.jp/cando/)

## Four-skill pass rule

Reading, listening, writing, and speaking are separate 100-point curriculum
papers. A candidate must score at least 60/100 on every paper in the same
administration. There is no aggregate compensation. Where a level has a JLPT
anchor, the receptive mock must additionally satisfy the corresponding official
overall and sectional conditions after documented calibration; the local 60%
threshold does not replace them.

Writing and speaking use four equally weighted analytic dimensions:

1. task fulfilment and relevant content;
2. comprehensibility, organisation, and interaction;
3. vocabulary, grammar, and register control;
4. orthographic control for writing, or pronunciation and fluency for speaking.

Each dimension is scored 0–5, then scaled to the paper total. Kana, kanji, and
punctuation expectations must be stated per task. A readable kana response is
not penalised for omitting kanji that the inventory does not require. Romaji is
accepted only when the task explicitly licenses it.

Before any “learner proven” claim, both complete timed mocks at the rung must be
passed and a preregistered book-only pilot must show each skill passing
independently. At least two trained raters double-score writing and speaking;
a third adjudicates material disagreement. JLPT-linked receptive forms require
review by a Japanese assessment specialist and empirical calibration against
official practice material or candidates with recent official scores.

## Administration and gentle-ramp rules

- Lessons remain five minutes or shorter. Full mocks preserve continuous exam
  timing because assessment endurance is a separate skill that the book must
  build gradually through partial mocks.
- Listening, speaking, and production prompts use contemporary standard
  Japanese while teaching learners to recognise common polite and plain
  registers. Regional forms are not treated as errors when a task licenses
  them and meaning remains clear.
- At pre-A1 and A1, directions may include a declared support language. From A2
  onward, Japanese directions include one unscored example.
- No dictionary, translator, grammar reference, spell-checker, or generative
  assistant is allowed unless an accessibility accommodation explicitly changes
  response mode without changing the construct.
- Official JLPT item wording is not copied into project mocks. Future task
  inventories reproduce the published construct and timing with original items.
- Writing starts with observing and tracing individual signs, then guided copy,
  delayed copy, dictation/transcription, controlled composition, connected
  composition, and finally timed independent production. A learner never jumps
  from recognising a sign to composing a paragraph.

## Level envelopes

### pre-A1

This project-defined precursor tests a tiny doorway exchange and the script
actions needed to enter N5 study. Reading and listening cover greetings, yes/no,
identity, and a few classroom cues. Writing samples isolated practised kana,
delayed copying, short dictation, and one independently recalled word or phrase.
Speaking covers a greeting, name, yes/no response, and one memorised request.
The assessed response is never longer than one short turn.

Required writing evidence: observe/trace, guided copy, delayed copy, and
dictation/transcription. Tracing and visible copying are teaching evidence, not
independent pass evidence.

### A1

Receptive anchor: N5, with a passing total of at least 80/180, combined language
knowledge and reading at least 38/120, and listening at least 19/60. The
production companion uses JF Standard A1 Can-dos: complete a small form, write
a 30–40 character personal message for a named reader, answer familiar personal
questions, and complete a simple rehearsed role-play.

### A2

Receptive anchor: N4, with a passing total of at least 90/180, combined language
knowledge and reading at least 38/120, and listening at least 19/60. A qualifying
N3 score of 95–103 is accepted as equivalent receptive evidence. The production
companion requires a short functional response, a connected 80–120 character
message or description, a two-minute account, and an information-exchange
role-play.

### B1

Receptive anchor: N3, with all three sections at least 19/60 and a total of at
least 104/180 for the official B1 reference. A qualifying N2 score of 90–111 is
also accepted. The production companion requires audience-aware correspondence,
a connected 250–350 character narrative or opinion, a prepared account,
collaborative planning, and follow-up discussion in an appropriate register.

### B2

Receptive anchor: N2, with all three sections at least 19/60 and a total of at
least 112/180. A qualifying N1 score of 100–141 is also accepted. The production
companion requires a practical interaction text, a structured 500–700 character
argument or report, a presentation, collaborative problem-solving, and defended
discussion that moves between polite and plain styles when the relationship
changes.

### C1

Receptive anchor: N1, with all three sections at least 19/60 and a total of at
least 142/180. The production companion requires synthesis of multiple sources,
an audience-aware 900–1,200 character argument or critical response, a
source-based presentation, collaborative synthesis, and sustained defence under
follow-up questions. Register, implication, and stance are scored explicitly.

### C2

There is no official JLPT C2 reference band. This project-defined extension
retains an N1-plus receptive floor, then adds dense specialist, public, and
literary reading; rapid multiparty and culturally implicit listening; precise
multi-source writing in contrasting genres; and sustained spoken mediation,
negotiation, reformulation, and defence. Task inventories must sample unfamiliar
domains so memorised specialist vocabulary cannot substitute for broad control.

## Required artifacts and readiness language

Every rung still needs:

1. four complete task-shape inventories with timing and input limits;
2. two original full timed mock forms;
3. shared analytic rubrics plus task-specific scoring notes;
4. answer keys, acceptable variants, and rater training samples;
5. calibration and double-scoring evidence;
6. a preregistered book-only human-validation report.

Until the first four exist, reports may say **target contracted** only. Passing
both calibrated mocks supports **mock ready**. Only the human study supports
**learner proven**. None of those phrases may be inferred from chapter count,
level labels, vocabulary coverage, or a JLPT result by itself.
