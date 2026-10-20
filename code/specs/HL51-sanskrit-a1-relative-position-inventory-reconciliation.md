# HL51 — Sanskrit A1 relative-position inventory reconciliation

## Status

Proposed for issue #17495.

## Problem

The Sanskrit A1 editorial exam inventory reports `SA-A1-SP-02` as uncovered
with `probe: null`. Its note says that the book teaches none of the six
relative-position exponents: left, right, above, below, in front, and behind.

That claim is stale. The lesson corpus already owns all six meanings, and a
later review retrieves them together. Because the point has no probe, the
coverage report and completion planner cannot see teaching that already exists.
They report one unit of content debt that must not be paid by duplicating the
lessons.

## Existing evidence

The point must use these exact existing owners, in learner order:

| Meaning | Taught form | Introducing lesson | Knowledge atom |
| --- | --- | --- | --- |
| left | **वामः** (*vāmaḥ*) | `SA-C93-vamah` | `SA-LEX-C93-TIME93-03` |
| right | **दक्षिणः** (*dakṣiṇaḥ*) | `SA-C93-daksinah` | `SA-LEX-C93-TIME93-04` |
| above | **उपरि** (*upari*) | `SA-C94-upari` | `SA-LEX-C94-LOC94-02` |
| below | **अधः** (*adhaḥ*) | `SA-C94-adhah` | `SA-LEX-C94-LOC94-03` |
| in front | **अग्रे** (*agre*) | `SA-C94-agre` | `SA-LEX-C94-LOC94-04` |
| behind | **पृष्ठतः** (*pṛṣṭhataḥ*) | `SA-C94-prsthatah` | `SA-LEX-C94-LOC94-05` |

`SA-R99-first-pass-time-and-places` names and assesses the six meanings in one
ordered retrieval. This is corroborating curriculum evidence; the inventory
probe remains the six introducing atoms because exam coverage measures whether
the corpus introduces every required exponent.

## Semantic correction

The stale note names **पश्चात्** (*paścāt*) for “behind”. The corpus teaches
that form in `SA-C92-pascat` only as temporal “after, later”. Counting that atom
for a spatial point would silently broaden its taught meaning.

The spatial owner is **पृष्ठतः** (*pṛṣṭhataḥ*), introduced and practised as
“behind” in chapter 94. The repaired note and probe must name that actual
evidence. No new sense claim is inferred from a shared surface form.

## Required change

1. Set `SA-A1-SP-02.probe` to the six exact atoms above.
2. Replace its stale note with an evidence account naming chapters 93–94 and
   the chapter-99 joint retrieval.
3. Pin the exact point id, label, ordered probe, covered status, and empty
   missing-atom list in a Sanskrit-owned test.
4. Raise the Sanskrit A1 coverage pin from 142/164 to 143/164 and reduce its
   unmapped count from 22 to 21.
5. Record the reconciliation in an additive Sanskrit changelog shard.

## Forbidden shortcuts

- Do not add lessons, chapters, curriculum nodes, or new atoms for material the
  book already teaches.
- Do not probe temporal `SA-LEX-C92-TIME92-03` for the spatial “behind” demand.
- Do not edit the inventory denominator or remove, merge, or weaken any point.
- Do not turn the partial editorial inventory into a completeness, attainment,
  readiness, qualification, or traditional-parīkṣā claim.
- Do not edit a cross-language aggregate; this correction belongs to the
  existing Sanskrit A1 inventory and Sanskrit-owned tests.

## Acceptance

- `measureExamCoverage` reports 143 covered, 21 unmapped, zero partial, and
  164 enumerated Sanskrit A1 points.
- `SA-A1-SP-02` is covered with no missing atoms and the exact six-owner probe.
- The exact completion plan reports 21, not 22, outstanding
  `exam-point/sanskrit/A1` points.
- Inventory census, Sanskrit corpus, progress, shard, doc-shard, lesson, and
  affected-package checks remain green.

This change reconciles measurement with existing teaching. It creates no new
learner evidence and makes no new readiness claim.
