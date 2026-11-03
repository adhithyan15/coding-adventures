# HL18889 — Marathi A2 curriculum-to-assessment boundary

**Status:** implementation specification, 2026-11-03

**Issue:** [#18889](https://github.com/adhithyan15/coding-adventures/issues/18889)

## 1. Evidence and decision

Marathi's authored level gate remains the end of regular chapter 275.  The
track already assigns chapters 159 and 160 to the A2 past and future spine
nodes, but those chapters currently own only vocabulary atoms.  Their prose
shows past and future examples without decomposing, practising, or assessing
the grammar that makes the examples work.  The later B1 seam says so
explicitly: chapters 276 and 277 bridge the A2 past and future that the earlier
track left empty before chapters 278–287 begin the five B1 spine nodes.

The published A2 task shape is not the defect.  Connected everyday accounts,
event descriptions, and functional interaction need a small productive past
and future, exactly as the core A2 spine declares.  Narrowing the assessment to
avoid those forms would hide the curriculum gap.  Moving the A2 gate past the
B1 seam would instead let later material bless an earlier assessment.

The repair is therefore a slow, explicit A2 grammar path inside the existing
chapters 159 and 160.  Chapter 159 gains three lessons: the intransitive past,
the transitive past with **-ने** and object agreement, and the past copula.
Chapter 160 gains two lessons: person-marked future forms and **-णार आहे / नाही**
for plans and refusals.  Chapters 276–277 remain later spiral reinforcement;
they are not source evidence for A2.

The repository evidence behind that decision is explicit and bounded:

- `core/spine.d/0210-SPINE-TALK-ABOUT-PAST.json` and
  `0220-SPINE-TALK-ABOUT-FUTURE.json` assign both capabilities to A2;
- `marathi/chapters.d/0159.json` and `0160.json` already claim those two spine
  nodes inside the regular A2 run;
- the pre-repair chapter 159–160 lessons own time vocabulary and examples but
  no productive past/future grammar atoms; and
- `marathi/CHANGELOG.d/00780-*.md`, chapters 276–277, and their lessons describe
  those later chapters as the bridge into the B1 core.  They are useful spiral
  evidence, but they are downstream of the declared A2 gate.

## 2. Machine-readable boundary

`marathi/assessment-boundaries/a2.json` is the track-owned source contract.  It
declares regular chapter 275 as the last admissible A2 chapter, chapter 276 as
the first excluded B1-seam chapter, the five owned grammar lessons, their ten
knowledge atoms, and the exact learner-facing forms that each lesson must make
visible.  The contract is intentionally narrower than a token dictionary: it
does not claim that a word occurrence alone proves productive grammar.

The regression must fail closed when:

1. the contract shape, language, level, or numeric boundary changes;
2. an evidence filename is not a regular `MR-C###-*` lesson at or before the
   declared gate;
3. an atom has zero or multiple direct owners, or its owner is not the declared
   evidence lesson;
4. an evidence lesson omits its bounded response activity or any declared
   learner-facing form; or
5. chapter 276 or later is admitted as A2 evidence.

This grammar contract complements the mock's token/source audit.  The mock
implementation must still trace its complete learner-facing vocabulary and
accepted responses to regular source lessons at or before chapter 275.

## 3. Gentle lesson requirements

Each new lesson stays below the five-minute ceiling, requires only earlier
knowledge, introduces no idiom, sense, or culture claim, and contains one
bounded text-response activity with explicit feedback.  Chapter 159 orders the
three lessons from subject agreement, through the transitive exception, to the
past copula.  Chapter 160 orders the person-marked future before the plan form.
The generated book and narration must include the new lessons in that order.

The chapter payoffs and A2 curriculum extensions must own the new grammar
atoms.  README and language-owned changelog text must distinguish first A2
instruction from the later B1-seam reinforcement.

## 4. Acceptance

- the focused boundary regression passes after first being demonstrated red;
- schema, corpus, curriculum, generated-book, narration, and integration checks
  pass without relaxing a gate;
- the completion planner is rebuilt and its exact effect is recorded;
- an independent exact-head security/quality review passes before publication.

This work establishes project-owned curriculum evidence only.  It does not
claim external certification, recording provenance, qualified recordings,
independent human validation, standard setting, learner readiness, or learner
attainment.
