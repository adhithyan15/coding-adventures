# HL55 — Sanskrit A1 meal ordering from owned words

## Status

Proposed for issue #17527 under #13415 and #13295.

## Problem

The project-defined Sanskrit A1 point `SA-A1-M-02` asks for meals, an eating
place, and ordering. Its inventory note says that the track teaches foodstuffs
but no meal, restaurant, or ordering language. The first two claims are stale:
chapter 97 owns **भोजनालयः** (*bhojanālayaḥ*, “a restaurant”), chapter 160
owns **भोजनम्** (*bhojanam*, “a meal, food”), and earlier chapters own eating,
food and drink, **मह्यम्** (*mahyam*, “to/for me”), **कृपया** (*kṛpayā*,
“please”), and **ददाति** (*dadāti*, “gives”).

The productive gap is narrower. Both A1 mocks use requests of the shape
**कृपया मह्यं जलं ददातु** (“Please give me water”), but no lesson introduces
**ददातु** (*dadātu*) or composes the complete request. Mapping isolated meal
and restaurant nouns would therefore overclaim functional coverage.

## Contract

Append one Sanskrit-owned chapter after chapter 264 with six lessons:

1. retrieve the already-owned meal, restaurant, eating, food, and drink atoms;
2. bridge from known **ददाति** to the complete form **ददातु** and use it as
   the polite third-person singular imperative required by the mock exchange;
3. compose **मह्यम्** + a familiar neuter item + **ददातु**;
4. add **कृपया** and contrast naming a meal with making an order; and
5. finish with two successive model-free mixed-ordering rounds in which the
   learner chooses a familiar item and produces the request independently.

The ramp teaches the whole usable request before it names the relationship
between **ददाति** and **ददातु**. Whitney, *Sanskrit Grammar* §§617 and 668–669
anchors the present imperative and the reduplicating root **दा**; the checked-in
A1 mock scripts anchor this project's polite spoken use. The lesson must not
turn one form into a complete imperative algorithm or paradigm.

## Knowledge ownership

The six lessons own exactly two new atoms:

- `SA-GRAMMAR-C265-DADATU-01`
- `SA-PATTERN-C265-POLITE-ORDER-02`

The first is introduced by the second lesson and the second by the third. Both
are retrieved by each remaining lesson. The last two lessons introduce no
atoms, require every point atom, and show no copyable Sanskrit answer before
the learner responds.

The exact `SA-A1-M-02` probe is:

- `SA-LEX-C97-LOC97-02`
- `SA-LEX-C160-THG160-01`
- `SA-LEX-KHADATI-EAT`
- `SA-LEX-C29-COURTESY-01`
- `SA-PRON-DATIVE-ME-01`
- `SA-LEX-C72-HOUSEWORK-05`
- `SA-LEX-ANNAM-FOOD`
- `SA-LEX-PANIYAM-WATER`
- `SA-LEX-KSHIRAM-MILK`
- `SA-GRAMMAR-C265-DADATU-01`
- `SA-PATTERN-C265-POLITE-ORDER-02`

## Reinforcement boundary

Appending six lessons makes six old atoms newly judgeable in each of R3 and
R4, and three chapter-264 atoms newly judgeable in R2. In lesson order, the
warm-ups must retrieve these pairs:

1. `SA-LEX-C247-THG247-03` and `SA-GRAMMAR-C259-PLURAL-POSSESSORS-03`, plus
   `SA-VERB-C264-KARISYAMI-01`;
2. `SA-LEX-C247-THG247-04` and `SA-PRON-C260-MAM-01`, plus
   `SA-VERB-C264-PATHISYAMI-02`;
3. `SA-LEX-C247-THG247-05` and `SA-PRON-C260-TVAM-02`, plus
   `SA-PATTERN-C264-NEAR-TERM-PLAN-03`;
4. `SA-LEX-C248-THG248-01` and `SA-GRAMMAR-C260-SINGULAR-OBJECTS-03`;
5. `SA-LEX-C248-THG248-02` and `SA-PRON-C261-AVAM-01`; and
6. `SA-LEX-C248-THG248-03` and `SA-PRON-C261-YUVAM-02`.

This boundary work is deliberately short recall, not new teaching. The new
atoms still need ordinary R1 revisits inside chapter 265.

## Ownership and sharding

Lesson files, chapter ledger, curriculum extension/path/membership owners,
book target, generated modality/book/narration owners, corpus evidence, the
operational lesson, and the Sanskrit changelog entry must be Sanskrit-owned or
single-record additive files. The inventory point and existing Sanskrit
aggregate pins are the only same-track files whose measured contract changes.
No cross-language mutable aggregate may be introduced.

## Evidence boundary

This slice closes one project-defined editorial A1 point. It proves one small
meal/eating-place request with familiar items. It does not prove complete
restaurant or nutrition coverage, a general imperative paradigm, B1 human
validation, exam readiness, learner success, or equivalence to a traditional
Sanskrit examination. The A1 inventory remains partial.

## Acceptance

- A Sanskrit-owned red-before-green test pins all six lessons, their order,
  exact ownership, prerequisites, duration ceilings, chapter payoff, and exact
  inventory probe.
- No owned meal, restaurant, food, drink, or request vocabulary is retaught.
- The first new form starts from **ददाति**; no learner infers an unseen stem.
- Every lesson stays within five minutes, and the final two introduce nothing.
- Learner copy states that **ददातु** is the taught request form here, not a
  complete imperative paradigm.
- Sanskrit A1 coverage moves from 146/164 to 147/164, with 17 unmapped and
  zero partial points.
- The exact completion plan reports 17 outstanding
  `exam-point/sanskrit/A1` points; its tranche count is measured after authoring.
- Corpus, curriculum, shard, document-shard, modality, generated-book,
  narration, lesson, figure, compile, affected-package, and full book checks
  pass.
- Independent security and data-integrity review passes on the exact pushed
  head before every push.
