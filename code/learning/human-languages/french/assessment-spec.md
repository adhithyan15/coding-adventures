# Coding Adventures French Assessment

**Version:** 1.0 target contract, 2026-08-20

**Basis:** official DILF, DELF *tout public*, and DALF targets

**Status:** target contracted; most task inventories, mocks, calibration, and
book-only human validation remain backlog

This specification names the examinations that the French book must eventually
prepare a book-only learner to pass. It does **not** claim that the current book
is exam-ready. The checked-in [assessment.json](assessment.json) is the
machine-readable contract. Paths in it name required future artifacts; a path is
a dependency, not evidence that the inventory, rubric, answer key, or mock has
already been written.

The external ladder is unusually complete:

| curriculum rung | external target |
|---|---|
| pre-A1 | DILF A1.1, the official initial diploma and closest available runway target |
| A1 | DELF A1 *tout public* |
| A2 | DELF A2 *tout public* |
| B1 | DELF B1 *tout public* |
| B2 | DELF B2 *tout public* |
| C1 | DALF C1 |
| C2 | DALF C2 |

DILF's published level is A1.1 rather than the Council of Europe's named
pre-A1 rung. This contract uses it as the first **external readiness target**;
it does not assert that the labels are identical. DILF is available only in
France and has candidate-eligibility rules. A learner outside that eligibility
boundary can still sit the project's original DILF-shaped mocks, but only an
eligible candidate at an approved centre can earn the diploma.

## Official evidence

France Éducation international describes DILF as a four-part exam at A1.1 with
listening, reading, speaking, and writing. It gives 70 of 100 points to oral
skills, requires 50/100 overall, and requires at least 35/70 across the oral
tests. DELF A1 through B2 and DALF C1 each score four papers out of 25: a
candidate needs 50/100 overall and at least 5/25 in every paper. DALF C2 combines
reception and production into one written and one oral paper, each out of 50; a
candidate needs 50/100 overall and at least 10/50 in each combined paper.

Sources checked 2026-08-20:

- [DILF overview and pass rule](https://www.france-education-international.fr/diplome/dilf?langue=en)
- [DELF A1 format and pass rule](https://www.france-education-international.fr/diplome/delf-tout-public/niveau-a1?langue=en)
- [DELF A2 format](https://www.france-education-international.fr/diplome/delf-tout-public/niveau-a2)
- [DELF B1 format](https://www.france-education-international.fr/diplome/delf-tout-public/niveau-b1)
- [DELF B2 format](https://www.france-education-international.fr/diplome/delf-tout-public/niveau-b2)
- [DALF C1 format](https://www.france-education-international.fr/article/dalf-c1?langue=en)
- [DALF C2 format](https://france-education-international.fr/en/article/dalf-c2)
- [Official DELF A1 examples and assessment grids](https://france-education-international.fr/diplome/delf-tout-public/niveau-a1/exemples-sujets?langue=fr)

The public format pages and official examples are the authority when a future
session changes task count or presentation. A dated task-shape inventory must
record the variant it reproduces; this overview must not be used to override a
newer official specification.

## Two pass rules, both required

An official-format mock must apply the awarding body's real arithmetic:

- DILF: at least 50/100 overall and at least 35/70 across the oral tests;
- DELF A1–B2 and DALF C1: at least 50/100 overall and no paper below 5/25;
- DALF C2: at least 50/100 overall and neither combined paper below 10/50.

Those eliminatory floors are not a safe curriculum-readiness margin. A learner
could earn 5/25 in writing and still pass DELF by compensating elsewhere. The
book therefore imposes an additional local rule: reading, listening, writing,
and speaking must each score at least **60/100** on both complete timed mocks at
the rung. There is no compensation in the local rule. For DALF C2, raters score
the receptive and productive evidence inside each integrated paper separately
for this diagnostic threshold, while the official 50-point paper score remains
the score used for the external pass calculation.

This stricter local rule is a readiness claim, not a new DELF/DALF scoring rule.
The real examination result remains authoritative.

Writing and speaking mocks use the current official grids wherever France
Éducation international publishes them. Project-specific analytic notes may
clarify task fulfilment, coherence, range and control, orthography,
pronunciation, fluency, and interaction, but may not silently replace the
official criteria.

## Administration and gentle ramp

- Every lesson remains five minutes or shorter. A full mock preserves the
  official continuous timing because endurance under exam conditions is part of
  the evidence, not a reason to lengthen lessons.
- Writing begins with observing and tracing, then guided copy, delayed copy,
  dictation/transcription, controlled composition, connected composition, and
  finally timed independent production. A learner never jumps from recognising
  an accent to writing a DALF synthesis.
- Pre-A1 and early A1 lessons may explain directions in a declared support
  language. Original mocks reproduce the language and support boundary of the
  external target they model.
- No dictionary, translator, grammar reference, spell-checker, or generative
  assistant is allowed unless the official paper permits it or a documented
  accessibility accommodation changes response mode without changing the
  construct.
- Speaking is recorded for local validation. Two trained raters independently
  score writing and speaking; a third adjudicates material disagreement.
- Public sample-paper wording is not copied into repository mocks. Future forms
  reproduce the published construct with original prompts and source texts.
- Two full mocks are a minimum, not the whole practice programme. Earlier
  five-minute lessons and partial mocks must build timing gradually before the
  learner first meets a continuous paper.

## Level envelopes

The future `task-shapes/<level>.json` files must make these envelopes executable
without widening their input, response, timing, interaction, or aid boundary.
When an official page publishes multiple live formats, the inventory must keep
the alternatives explicit rather than blending them into a form that does not
exist.

### pre-A1

The target is DILF A1.1: four skills, 1 hour 15 minutes in total, with oral
reception and production weighted more heavily than the written tests. The
learner must handle public announcements and highly familiar spoken exchanges,
extract practical details from very short written material, produce rehearsed
personal and transactional speech, and write the small personal or practical
responses the official specification requires.

Before the first timed mock, the learning record must prove observe/trace,
guided copy, delayed copy, and dictation/transcription. Tracing and visible
copying are instructional evidence; only responses produced under the mock's
independence boundary earn exam points.

### A1

DELF A1 has four listening exercises in 20 minutes, four reading exercises in
30 minutes, and a 30-minute writing paper that completes a form and writes a
message of at least 40 words. The 5–7 minute speaking paper has a directed
interview, information exchange, and simulated dialogue after 10 minutes of
preparation.

The existing [A1 task-shape inventory](task-shapes/a1.json) records the official
formats, sources, unknowns, scoring floor, prompt modes, and aids in machine-
readable detail. Future mocks must conform to that inventory rather than to this
short summary.

### A2

DELF A2 has four listening exercises in 25 minutes, four reading exercises in
30 minutes, and two writing tasks of at least 60 words each in 45 minutes. The
6–8 minute speaking paper contains a directed interview, sustained monologue,
and interaction task after 10 minutes of preparation. Writing must already be
connected and audience-aware; copied phrases cannot substitute for either
independent 60-word response.

### B1

DELF B1 has three listening exercises in 25 minutes, reading in 45 minutes, and
one independent written response of at least 160 words in 45 minutes. The
15-minute speaking paper combines a directed interview, a defended point of
view, and interaction after 10 minutes of preparation. The book must therefore
build narration, justification, repair, and unscripted interaction—not merely
recognition of B1-labelled vocabulary.

### B2

DELF B2 has listening in 30 minutes, reading in 60 minutes, and one argued
written response of at least 250 words in 60 minutes. The 20-minute speaking
paper requires a sustained viewpoint and interaction after 30 minutes of
preparation. Register, organisation, evidence, and defence under follow-up
questions must all appear in the lesson sequence before mock work begins.

### C1

DALF C1 has listening in 40 minutes, reading in 50 minutes, a 2 hour 30 minute
writing paper, and a 30-minute oral after 1 hour of preparation. Writing combines
a 220–240 word synthesis with a separate text of at least 250 words. Speaking
combines a monologue and interaction. The synthesis must preserve source
relationships without copying; the second text must adopt the requested genre,
audience, and stance.

### C2

DALF C2 integrates reading with writing and listening with speaking. The written
paper gives 3 hours 30 minutes to produce a structured text of at least 700 words
from an approximately 2,000-word dossier. The oral paper gives 1 hour of
preparation and 30 minutes for an account of a twice-heard recording, personal
development of its issue, and debate with the jury.

The book must teach the integration itself: selecting and reorganising source
material, distinguishing source claims from the learner's contribution,
reformulating under pressure, recovering from challenge, and sustaining precise
register and implication across unfamiliar domains.

## Required artifacts and readiness language

Every rung still requires:

1. a dated four-skill task-shape inventory (French A1 is the only one already
   present);
2. two original, complete, timed mock forms;
3. official-grid-aligned rubrics and task-specific scoring notes;
4. answer keys, acceptable variants, and rater training samples;
5. documented calibration against official examples and qualified raters; and
6. a preregistered book-only human-validation report with every skill reported
   separately.

Until the first four exist, reports may say **target contracted** only. Passing
both calibrated mocks supports **mock ready**. Only the human study supports
**learner proven**. None of those phrases may be inferred from chapter count,
vocabulary count, CEFR labels, or the presence of this file.
