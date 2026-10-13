# HL43 — Direct-owner corpus invariants

**Status:** specification, 2026-10-13

**Tracks:** #13193, #17417

---

## 1. Problem

Curriculum lessons, chapter membership, and reconstructed session-map sections
already have stable direct owners. A test that repeats their assembled length or
copies every assembled observation into one expectation creates a second owner:
every independent author must update the same test even when the invariant did
not change.

The HL42 report measured this failure in Punjabi. Two corpus tests were touched
by 22 of the newest 200 scoped human-language commits. Two more were touched by
15 and 14. Their shared edits were generated row/lesson totals, a whole-track
continuity snapshot, a whole-track writing-evidence array, and cumulative
historical narration. None was the stable semantic claim named by its test.

## 2. Rule

A corpus test must assert the smallest stable semantic claim whose ownership it
names. It must not pin an assembled count or sequence when direct owners and a
general validator already prove completeness.

For the current Punjabi seams:

- the session-map test proves contiguous row numbering and exact chapter/lesson
  identity against the assembled canonical lessons; its expected length is
  derived from those compared rows, never a numeric literal;
- the lesson-budget test lets `expectLanguageLessonBudgets` derive the lesson
  count from canonical schema-v2 lessons; stable idiom, sense, and culture-claim
  totals remain because they are semantic inventory claims rather than a
  generated lesson count;
- the Chapter 3 continuity test proves that its named Chapter 3 atoms have no
  remaining reinforcement defect; it does not own the changing whole-track
  `missedByWindow` summary;
- the pre-A1 writing-runway test proves zero defects, complete required stages,
  and the stable first evidence owner for each required stage; later valid
  repetitions are not copied into an ever-growing array.

Historical tranche accounting belongs in the uniquely owned changelog or issue
that introduced the tranche. It does not stay as an append-only comment inside
an invariant test.

## 3. Strength preserved

Removing generated totals must not make deletion or reordering invisible.

- Session rows are still compared one-for-one with canonical lessons. A missing,
  extra, duplicated, renumbered, or reordered row fails.
- Lesson budgets still require every schema-v2 lesson to be measured, every unit
  id to use the Punjabi prefix, and every per-lesson policy excess to be absent.
- Chapter 3 still fails on any named atom/window debt.
- The writing runway still fails on invalid evidence, a missing prerequisite,
  an incomplete pre-A1 stage set, or movement of a first-stage anchor.

The change removes redundant expected data, not validation.

## 4. Regression contract

A focused test reads the four formerly hot Punjabi owners and rejects the
specific whole-corpus counter shapes retired here: numeric session-map lengths,
an explicit Punjabi `lessons` total, a whole-track `missedByWindow` equality,
and an exact ordered array of every writing-evidence stage. This source-level
guard is intentionally narrow; tranche-specific before/after continuity tests
may still pin the measurements that define their own direct claim.

Appending an independent Punjabi chapter may add lesson, membership,
session-map, and changelog owners. If its semantic inventory totals do not
change, none of these four invariant owners should require an edit.

No replacement tracked aggregate, digest, expected-count file, or generated
snapshot is permitted.

## 5. Verification

The implementation must run:

- the focused direct-owner regression;
- the complete Punjabi corpus suite;
- the human-language-data TypeScript build and full test suite;
- the HL42 report after merge, recording the new priority on #13193.
