# HL18936 — Kannada B2 assessment-inventory closure

## Problem

The Kannada B2 inventory enumerates 36 reviewable demands, nine in each HL20
dimension, but still describes every dimension as partial.  That leaves the
stable `exam-inventory/kannada/B2` item open even though the repository already
has a finite project-defined task contract and a finite content selection.

The external anchors do not publish a finite Kannada B2 grammar or word list.
They must not be represented as doing so.  This project may nevertheless close
its own inventory boundary when it says exactly what is inside that boundary,
why each external source is used, and what closure does *not* prove.

## Source decision

- The Council of Europe CEFR Companion Volume supplies generic B2 performance
  descriptors, not Kannada exponents and not certification of this assessment.
- The cited CIIL intermediate Kannada text supplies a four-skill instructional
  reference, not a finite external B2 syllabus.
- Unicode 18.0 section 12.8 closes only the Kannada encoding and rendering
  model.  It does not close spelling, handwriting, pronunciation, or fluency.
- The official LDC-IL documentation supplies contemporary genre domains, not a
  level-bounded lemma list or frequency threshold.
- The published Kannada B2 task shape fixes the performances this repository
  samples.  It does not by itself prove that any content has been taught.

The correct repair is therefore a transparent **project-owned finite
boundary**: the 36 existing demands are the complete content inventory for this
repository's Kannada B2 assessment contract.  Each of the four dimensions is
complete *as an inventory*.  Lesson coverage remains measured separately and
strictly; a null probe is still uncovered work, and inventory closure is not
learner readiness, attainment, external validation, or external certification.

## Contract

1. Keep the 36 stable point IDs and the nine-per-dimension split.
2. Mark all four scope dimensions complete only after their notes explicitly
   name the finite project-owned boundary and its source limitations.
3. Keep strict coverage at 0/36 until full labeled demands have exact Kannada
   lesson-atom probes.  Foundation subsets do not count.
4. Keep the task shape, inventory, and lesson-coverage measurements separate.
5. The completion planner must remove `exam-inventory/kannada/B2`, while the 36
   uncovered points remain visible as `exam-point` work.
6. Documentation must avoid claims of standard setting, qualified human
   validation, recording provenance, learner readiness, or learner attainment.

## Validation

Demonstrate the old partial inventory failing the new contract, then run the
focused Kannada inventory and planner tests, the inventory census, integration
tests, build/type checking, assessment-artifact and shard gates, book checks,
and the freshly built completion planner.  Measure the planner effect rather
than assuming it.
