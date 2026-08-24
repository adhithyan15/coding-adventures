# Coding Adventures Spanish Assessment

**Version:** 1.0 target contract, 2026-08-24

**Basis:** official DELE targets (Instituto Cervantes), with a project-defined
pre-A1 runway

**Status:** target contracted; most task inventories, mocks, calibration, and
book-only human validation remain backlog

This specification names the examinations that the Spanish book must eventually
prepare a book-only learner to pass. It does **not** claim that the current book
is exam-ready. The checked-in [assessment.json](assessment.json) is the
machine-readable contract. Paths in it name required future artifacts; a path is
a dependency, not evidence that the inventory, rubric, answer key, or mock has
already been written.

The external ladder is CEFR-identity, and `core/exam-levels.json` already
records it that way (`exam: DELE`, `basis: published`, `mapping: identity`):

| curriculum rung | external target |
|---|---|
| pre-A1 | **project-defined** — no external certificate exists at this rung |
| A1 | DELE A1 |
| A2 | DELE A2 |
| B1 | DELE B1 |
| B2 | DELE B2 |
| C1 | DELE C1 |
| C2 | DELE C2 |

## Why pre-A1 is project-defined here, and not in French

The DELE ladder starts at **A1**. The Instituto Cervantes publishes six diplomas
— A1, A2, B1, B2, C1, C2 — and nothing below A1. The school-age variants (A1
para escolares, A2/B1 para escolares) are age-adapted forms of those same
levels, **not** sub-levels.

This is a real asymmetry with French, and it is worth stating rather than
papering over. French can point pre-A1 at DILF A1.1, an official diploma that
exists precisely because the French system wanted a rung below DELF A1. Spanish
has no counterpart. So Spanish follows HL18 §2 — *"There is no external pre-A1
certificate in this model, so that rung uses a clearly labelled project-defined
equivalent"* — and the pre-A1 target is named, in full, as a project artifact.

pre-A1 is also not a CEFR level. `core/exam-levels.json` says so directly: it is
this curriculum's own label for the ramp below A1, the greetings-and-first-words
stretch that an exam syllabus assumes you already have. Naming it keeps the ramp
measurable instead of invisible.

## Official evidence

The Instituto Cervantes publishes, for DELE A1: a 95-minute written session and
a 10-minute individual speaking test with 10 minutes of preparation, sat on
paper at an accredited examination centre. Scoring is out of 100, with 25 points
per skill, and the award is **APTO / NO APTO** rather than a mark.

The pass rule is **grouped, not per-skill**: reading plus writing form one group
and listening plus speaking the other, each group must reach at least 30/50, and
the overall floor is 60/100. There is no independently published minimum for a
single skill, which is why
[`task-shapes/a1.json`](task-shapes/a1.json) records
`independentSkillThresholds` as `null` for all four rather than inventing one.

Those figures are recorded, with five Cervantes sources and an `accessed` date,
in the existing A1 inventory. **Only A1 has been inventoried.** The grouped
two-block structure and the 60/100 floor are DELE-wide, but per-level timings,
exercise counts, and word minimums for A2 through C2 have not been transcribed
from the official guides yet, and this document does not estimate them. Each
level gets its own `task-shapes/<level>.json` before any claim is made about it.

## Two pass rules, both required

1. **The awarding body's rule**, recorded verbatim per level in the task-shape
   inventory. For DELE that is the grouped 30/50 + 60/100 rule above.
2. **The project's readiness rule**, which is stricter: every one of the four
   skills must independently reach 60%, across two timed full mocks, before the
   book may describe a learner as ready for that level.

The project rule is stricter on purpose. A grouped rule lets a strong reader
carry a weak writer; a book that produced that learner would have taught only
half its promise. Where the two disagree, the stricter one governs what the book
may claim — never what the diploma actually requires, which is reported as the
awarding body states it.

## Level envelopes

The `task-shapes/<level>.json` files must make these envelopes executable
without widening their input, response, timing, interaction, or aid boundary.

### pre-A1

The target is the **Coding Adventures Spanish pre-A1 Assessment**, a
project-defined DELE A1 precursor. Four skills, small and slow: recognise and
respond to fixed greetings and courtesy formulas, give and ask a name, say how
you are, read a handful of very short practical labels, and write single known
words and memorised chunks.

Before the first timed mock, the learning record must prove **observe/trace,
guided copy, delayed copy, and dictation/transcription**. Tracing and visible
copying are instructional evidence only; under the mock's independence boundary,
they earn no points. Scored pre-A1 writing is therefore limited to delayed
recall, dictation/transcription, and bounded independent production — the three
shapes HL18 §2 requires at this rung.

Spanish's runway for this is `ES-W00-hola-*`: four micro-lessons that take one
already-known word from tracing to writing-from-sound, using the silent **h** as
the thing the ear cannot supply.

### A1

DELE A1 is the first external diploma and the first rung where the book's claim
is checkable against a published format. The
[A1 task-shape inventory](task-shapes/a1.json) records the official
administration, grouped pass rule, prompt modes, response modes, aids, and every
measurement Cervantes does not publish. It is currently **partial**: completing
it is tracked as its own backlog item, and until it is complete every Spanish
number at A1 is a proxy.

Future A1 mocks must conform to that inventory rather than to this summary.

### A2 through C2

Not yet inventoried. Each rung needs its own `task-shapes/<level>.json`
transcribed from the official Cervantes guide and specification for that level,
with sources and access dates, before this document says anything more specific
than the DELE-wide grouped pass rule.

Deliberately left blank rather than filled with plausible-looking estimates.
HL18 §3 makes unknown first-class: a measurement the source does not publish is
`null` plus a `notPublished` explanation, never a guess.

## Required artifacts and readiness language

Per level, before the book may claim readiness:

- a complete `task-shapes/<level>.json` covering all four skills;
- a complete exam inventory in `core/exam-inventory-es-<level>.json`;
- the cumulative writing stages for that level proved in the lesson record;
- rubrics and answer keys for every scored task;
- two timed full mocks, each passed at ≥60% per skill independently.

Until all of those exist for a level, the book and every generated report say
**in progress at &lt;level&gt;**, never *reaches* or *prepares you for* it. That
wording rule is not cosmetic: `levels ATTAINED` is a derived, gated number, and
prose that outruns it would be the one claim this whole apparatus exists to stop.
