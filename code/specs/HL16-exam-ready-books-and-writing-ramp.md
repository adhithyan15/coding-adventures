# HL16 — Exam-ready books and the gentle writing ramp

**Status:** specification, 2026-08-20

**Extends:** HL09, HL10, HL11, and HL15

**Durable program issue:** [#12206](https://github.com/adhithyan15/coding-adventures/issues/12206)

## 1. The promise

Finishing a human-language book must prepare a learner to **pass the named
assessment**, not merely encounter material carrying the same level label. Where
an appropriate external examination exists, its current public specification is
the target. Where none exists, the project publishes a clearly labelled
project-defined equivalent with the same evidence, scoring, and quality bar.

This promise applies independently to reading, listening, writing, and speaking.
A strong reading score may not compensate for an absent writing course. A corpus
coverage report may not substitute for a scored learner performance. “Touches
C2” and “can pass C2” are different claims forever.

Every instructional lesson remains designed for **at most five minutes**. There
is no maximum book length. If an honest gentle ramp needs 50,000 micro-lessons,
the book has 50,000 micro-lessons.

## 2. Why HL15's six criteria are necessary and insufficient

HL15 measures concepts, vocabulary, atom budget, reinforcement, exam points, and
script closure. Those are strong corpus prerequisites. They do not establish the
shape of performance:

- recognising a word does not prove the learner can retrieve and write it;
- covering a grammar point does not prove the learner can use it under time;
- narration does not prove listening at the target speed and acoustic variety;
- one text activity does not reproduce a listening paper, oral interview, or
  extended writing task;
- an inventory says what is tested, not whether practice and scoring match the
  actual test;
- a perfect corpus cannot prove that a human who used only that corpus passes.

The Council of Europe's CEFR Companion Volume treats written production,
written interaction, spoken production, spoken interaction, reception, and
mediation as distinct activities. Its overall written-production ramp moves from
basic personal information at pre-A1, through isolated sentences at A1 and
connected text at B1, to complex, audience-aware writing at C1/C2. The project's
writing ramp follows that progression instead of bolting handwriting onto a
reading course.

Sources:

- [CEFR Companion Volume, communicative activities and written production](https://www.coe.int/en/web/common-european-framework-reference-languages/cefr-descriptors)
- [CEFR language-by-language Reference Level Descriptions](https://www.coe.int/en/web/common-european-framework-reference-languages/reference-level-descriptions)
- [ALTE assessment guides and separate-skill recommendations](https://www.alte.org/Materials)

## 3. The universal assessment policy

`core/assessment-policy.json` is the non-negotiable project contract. It records:

- the five-minute instructional ceiling;
- the seven curriculum levels from pre-A1 through C2;
- the four independently passing skills;
- the ordered writing stages;
- the minimum full-mock count;
- required timing, rubrics, answer keys, and human validation.

The data package parses this file strictly. A typo must fail loudly; it must not
silently relax the promise.

## 4. One assessment contract per track

Every registered track must eventually carry `<track>/assessment.json`. Absence
is an `assessment-contract` work item in the generated completion plan. A valid
contract contains every level and, at each level:

1. the target assessment name;
2. `basis: external` or `basis: project-defined`;
3. a stable source or checked-in project specification;
4. task inventories for reading, listening, writing, and speaking;
5. an independent pass threshold for each skill;
6. the writing stages required at that level;
7. at least two complete timed mocks;
8. a rubric and answer key for every mock.

“No widely-sat ladder” is no longer a terminal answer. It selects the
project-defined path. The equivalent must be published and reviewable; it may
not be an unnamed editorial feeling hidden in code.

Every contract reference to `task-shapes/<level>.json` is loadable and measured,
including pre-A1. Pre-A1 is not an external certificate and therefore does not
create an `exam-inventory` item, but its project-defined task shape must still
score reading, listening, writing, and speaking independently. A referenced but
absent file remains backlog; an unreadable or one-skill-only file fails closed.

### 4.1 External target

Use an external target only when the awarding body publishes enough information
to reconstruct the tested construct and scoring. Preserve edition/date, source
URLs, paper structure, timing, scoring, skill aggregation rules, permitted aids,
and sample/past-paper provenance. If the exam omits a skill, the book must still
teach that skill to the level descriptor, but must label the additional project
paper rather than pretending the external exam tests it.

### 4.2 Project-defined equivalent

A project-defined assessment must be a close functional approximation, not an
easier placeholder. Its specification must include:

- CEFR can-do descriptors or another declared proficiency framework;
- language-specific vocabulary, grammar, orthography, register, and genre
  inventories grounded in grammars, corpora, and community review;
- four separately scored papers;
- task counts, lengths, input speeds, interlocutor protocol, and timing;
- analytic rubrics and standard-setting rationale;
- two public mocks plus protected validation forms;
- an explicit pass rule that cannot hide a failed skill in an average;
- pilot evidence, reviewer qualifications, versioning, and known limitations.

The label shown to learners is “Coding Adventures <Language> <Level>
Assessment — project-defined equivalent,” never the name of an official
qualification the project does not award.

## 5. Writing is a full strand, beginning on the first page

Writing has two coupled ramps:

- **formation/orthography:** make the marks legibly and conventionally;
- **composition:** retrieve and arrange language for a reader and purpose.

Roman-alphabet tracks are not exempt. A Spanish learner still needs accents,
punctuation, spelling, paragraphing, register, and timed composition. A fluent
Devanagari reader learning Marwadi may skip detachable formation support, but may
not skip Marwadi spelling and composition evidence.

### Stage W0 — observe and trace

The model stays visible. The lesson names direction, joining, spacing, or the one
shape contrast that matters. The learner traces once or twice. No memory claim.

### Stage W1 — guided copy

The model remains visible while the learner copies one glyph, word, or phrase.
Formation cues fade one at a time. The learner compares against a concrete
checklist rather than “looks good.”

### Stage W2 — delayed copy

Look, briefly hide, write, reveal, compare, repair. Delay grows from one glyph to
one word and then one short phrase. This is the bridge from motor imitation to
orthographic recall.

### Stage W3 — dictation and transcription

Write a heard item, or convert a deliberately temporary romanized cue into the
target script. Begin with already-mastered contrasts at slow speed. Remove
romanization before it can become the learner's primary representation.

### Stage W4 — controlled composition

Choose known language to make a new answer: fill a real form, label a picture,
write personal details, order sentence pieces, then write an original sentence.
The response is no longer copied.

### Stage W5 — connected composition

Write connected sentences for a named purpose and reader: message, note,
description, narration, explanation, argument, synthesis. Planning, drafting,
revision, cohesion, register, and proofreading each receive their own five-minute
lessons before being combined.

### Stage W6 — timed assessment production

Rehearse the exact task family, length, tools, rubric, and time pressure of the
target assessment. Timing ramps gently: untimed with a model, untimed without a
model, generous partial timing, target timing, then full-paper timing.

Every stage is cumulative. C2 does not abandon legibility and proofreading; it
requires them while adding precision, genre control, synthesis, and speed.

## 6. The five-minute lesson grammar

A lesson must have one small observable outcome. Productive lessons use this
shape:

1. **Recall (30–60 seconds):** retrieve one prerequisite.
2. **Notice (30–60 seconds):** see or hear one new contrast.
3. **Model (up to 60 seconds):** study one worked response.
4. **Produce (up to 120 seconds):** write or say one bounded response.
5. **Check and repair (up to 60 seconds):** compare against an answer or rubric
   and correct immediately.

Long performances are assessments or chapter payoffs assembled from previously
learned pieces. Their preparation remains micro-lessons. A full mock is allowed
to take the real exam's duration because it is evidence, not an instructional
lesson; the book must label that distinction.

## 7. Passing evidence

The completion claim has four layers, none substitutable for another:

| layer | question | required evidence |
|---|---|---|
| corpus coverage | Is every needed thing taught gently and reinforced? | gates, inventories, script and writing closure |
| task readiness | Has every exam task shape been practised and scored? | task inventory and checkpoints |
| simulation | Can the learner pass complete assessments under authentic conditions? | two timed mocks, independent skill thresholds |
| external validity | Does book-only study work for people? | human pilot against a real or project-defined assessment |

A track-level C2 “done” flag requires all four layers at every lower level. Until
the human layer is complete, reports must say **corpus ready** or **mock ready**,
never **learner proven**.

## 8. Generated backlog families

HL15 remains the ordering engine. HL16 adds these deficit families in successive,
testable tranches:

| family | one item means | projectability |
|---|---|---|
| `assessment-contract` | author/repair one track's seven-level target and pass contract | exact: one per track |
| `writing-stage` | close one track/level/stage coverage deficit | exact after manifests exist |
| `task-shape` | add missing teaching and scored practice for one paper task | exact after inventories exist |
| `mock-assessment` | add or repair one full timed mock and its scoring materials | exact: target count minus valid mocks |
| `human-validation` | run or repair one pre-registered book-only learner study | not honestly projectable before recruitment |

The first family is implemented with this spec. The others stay explicit here
and in #12207 until their validators land. A prose-only future family must never
be counted as zero.

## 9. Program sequence

This is a multi-day, likely multi-year corpus program. The sequence is a
dependency order, not a promise that later phases wait for every language:

### Phase A — make the target executable

- land this policy and the assessment-contract queue family;
- author one contract per track, including Marwadi;
- date and source every external target;
- specify project-defined equivalents where required;
- implement validators for writing stages, task shapes, mocks, and rubrics.

### Phase B — repair every book's foundation

- inventory current writing instruction by track, level, and W0–W6 stage;
- add formation and orthography micro-lessons from the first useful words;
- add delayed recall, dictation, and controlled composition;
- keep every lesson within the atom and five-minute budgets;
- regenerate every standalone book from canonical lessons.

### Phase C — build four-skill level ladders

- author external or project language inventories for A1 through C2;
- implement authentic reading and listening input families;
- implement interactive and sustained speaking tasks;
- implement functional, connected, and genre-specific writing tasks;
- add independent scoring and remediation loops at every level.

### Phase D — simulate the assessments

- build at least two complete public mocks per track and level;
- reproduce timing, sequencing, input speed, response constraints, and scoring;
- calibrate rubrics with double-scored samples;
- add targeted five-minute repair paths from every rubric dimension.

### Phase E — establish human evidence

- pre-register a book-only study protocol;
- recruit learners with documented starting profiles;
- require completion evidence rather than self-reported exposure;
- administer real exams where accessible and project equivalents elsewhere;
- report every skill score, attrition, accommodations, and confidence interval;
- revise the book and repeat when a skill misses its pass target.

### Phase F — maintain validity

- recheck awarding-body specifications on a scheduled cadence;
- version contracts and mocks when exams change;
- require community and expert review for under-resourced languages;
- recompute the generated queue after every merge;
- treat newly discovered work as a new measurable family before selecting the
  next tranche.

## 10. Immediate acceptance criteria

This specification's first implementation tranche is complete when:

- `core/assessment-policy.json` exists and is parsed strictly;
- its four skills, seven writing stages, five-minute ceiling, two-mock minimum,
  rubrics, answer keys, timing, and human-validation requirements are tested;
- malformed track contracts fail loudly;
- every track without a valid contract receives an `assessment-contract` item;
- that deficit appears in both the queue head and the projection;
- HL15 no longer describes production/task-shape work as an unowned late gap.

It does **not** close the program. It makes the program difficult to accidentally
declare complete.
