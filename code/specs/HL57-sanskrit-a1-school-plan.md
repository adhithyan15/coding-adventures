# HL57 — Sanskrit A1 school plan from owned words

## Status

Proposed for issue #17542 under #13415 and #13295.

## Problem

The project-defined Sanskrit A1 point `SA-A1-M-03` says that school,
timetable, examination, marks, and classroom language are absent. That note is
stale. The corpus already owns **विद्यालयः** (*vidyālayaḥ*, “school”),
**छात्रः** (*chātraḥ*, “student”), **अध्यापकः** (*adhyāpakaḥ*, “teacher”),
**पाठः** (*pāṭhaḥ*, “lesson”), **परीक्षा** (*parīkṣā*, “exam”),
**समयसारणी** (*samayasāraṇī*, “timetable”), **कक्षा** (*kakṣā*,
“class/classroom”), **अंकः** (*aṅkaḥ*, “mark/number”), **गृहकार्यम्**
(*gṛhakāryam*, “homework”), and **उत्तरम्** (*uttaram*, “answer”).

The remaining gap is composition. Those words live in isolated vocabulary
lessons. The learner has not yet used the known future language to ask when an
exam will be, answer from a small timetable, or state a homework/study plan.
Mapping the nouns alone would therefore overclaim functional coverage.

## Contract

Append one Sanskrit-owned chapter after chapter 266 with six lessons:

1. retrieve the ten already-owned school-institution words;
2. compose **परीक्षा कदा भविष्यति?** and answer **श्वः परीक्षा भविष्यति।**,
   then introduce **गृहकार्यं करिष्यामि।** and **पाठं पठिष्यामि।** as complete,
   bounded school-plan frames;
3. retrieve the two complete homework/study frames without deriving a noun
   declension or a future paradigm;
4. join the timetable, exam-time answer, homework plan, and study plan;
5. complete a first model-free school-day plan; and
6. complete a second model-free plan from a different familiar prompt.

The ramp adds no school vocabulary. It teaches one usable composition pattern
from complete forms the learner already owns and does not generalise a new
future paradigm.

## Knowledge ownership

The chapter owns exactly one new atom:

- `SA-PATTERN-C267-SCHOOL-PLAN-01`

The second lesson introduces it at the block where all four complete turns are
first shown. In particular, the atom owns **पाठं पठिष्यामि।** as a whole frame;
the lesson does not claim that **पाठं** was previously practised or generalise
its masculine accusative shape. Every later lesson retrieves the atom. The last
two lessons introduce no atoms, require and practise every point atom, and show
no copyable Sanskrit answer before the learner responds.

The exact `SA-A1-M-03` probe is:

- `SA-LEX-C83-SCHOOL-01`
- `SA-LEX-C83-SCHOOL-02`
- `SA-LEX-C83-SCHOOL-03`
- `SA-LEX-C83-SCHOOL-04`
- `SA-LEX-C83-SCHOOL-05`
- `SA-LEX-C140-READ140-02`
- `SA-LEX-C168-SCH168-01`
- `SA-LEX-C169-SCH169-02`
- `SA-LEX-C255-SCH255-04`
- `SA-LEX-C255-SCH255-05`
- `SA-LEX-C18-ASK-01`
- `SA-LEX-C139-FUT139-02`
- `SA-LEX-C139-FUT139-04`
- `SA-VERB-C264-KARISYAMI-01`
- `SA-VERB-C264-PATHISYAMI-02`
- `SA-PATTERN-C267-SCHOOL-PLAN-01`

## Reinforcement boundary

Appending six lessons makes older atoms newly judgeable in R2, R3, and R4.
In lesson order, the warm-ups must retrieve these groups at the first valid
position:

1. `SA-LEX-C249-THG249-05`,
   `SA-GRAMMAR-C263-FUTURE-FIRST-SINGULAR-02`, and
   `SA-VERB-C266-KRINAMI-01`;
2. `SA-PATTERN-C266-MARKET-EXCHANGE-02`;
3. no newly judgeable atom;
4. `SA-LEX-C250-THG250-01` and `SA-VERB-C264-KARISYAMI-01`;
5. `SA-LEX-C250-THG250-02` and `SA-VERB-C264-PATHISYAMI-02`; and
6. `SA-LEX-C250-THG250-03` and
   `SA-PATTERN-C264-NEAR-TERM-PLAN-03`.

This boundary work is short recall, not new teaching. The new school-plan atom
must still receive ordinary R1 revisits inside chapter 267.

## Ownership and sharding

Lesson files, chapter ledger, curriculum extension/path/membership owners,
book target, generated modality/book/narration owners, corpus evidence, and the
Sanskrit changelog entry must be Sanskrit-owned or single-record additive
files. The one inventory point and existing Sanskrit aggregate pins are the
only same-track files whose measured contract changes. No cross-language
mutable aggregate may be introduced.

## Evidence boundary

This slice closes one project-defined editorial A1 point. It proves one small
school-day plan with already-owned school and future language. It does not
prove complete classroom discourse, school-system knowledge, grading,
educational rights, future-paradigm coverage, B1 human validation, exam
readiness, learner success, or equivalence to a traditional Sanskrit
examination. The A1 inventory remains partial.

## Acceptance

- A Sanskrit-owned red-before-green test pins all six lessons, their order,
  exact ownership, duration ceilings, chapter payoff, and exact inventory
  probe.
- No owned school, timetable, examination, mark, classroom, homework, or
  answer vocabulary is retaught.
- Every lesson stays within five minutes, and the final two introduce nothing.
- Both final prompts say that no Sanskrit model is shown before the learner
  answers.
- Sanskrit A1 coverage moves from 149/164 to 150/164, with 14 unmapped and
  zero partial points.
- Corpus, curriculum, shard, document-shard, modality, generated-book,
  narration, lesson, affected-package, and full book checks pass.
- Independent security and data-integrity review passes on the exact pushed
  head before every push.
