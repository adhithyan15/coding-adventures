# HL-C310 — de-pin the corpus exam-point total in `plan-cli.test.ts`

**Status:** open
**Found:** 2026-09-02, during parallel human-language authoring

## The problem

`tests/plan-cli.test.ts` pins an absolute corpus-wide figure:

```
expect(out).toMatch(/774 uncovered point\(s\) across 8 written/);
```

In one day it moved **529 -> 686 -> 793 -> 792 -> 775 -> 774** across four
branches. It rises when an exam inventory lands (Telugu +157, Tamil +107) and
falls when a tranche covers points (Hindi -18) or a retirement closes one
(French -1).

Every parallel author must edit the same line, so it produced **three separate
DIRTY states on one PR** (#14088) and two more on #14097. Worse than the churn:
two branches that both lower it **merge quietly because they agree**, and the
agreed value is wrong. Every attempt to compose it by arithmetic today was wrong
— 793 and 668 were both defensible and both false; the tree said 775.

## Why a ratchet is NOT the fix

`toBeLessThanOrEqual` is wrong here: the number **legitimately rises** when a new
inventory lands, because an inventory enumerates points that were previously
unmeasurable. A ceiling would fail on exactly the work we most want to happen.

## Candidate shapes

1. Assert the **shape** plus an independently-derived inventory count: match
   `/\d+ uncovered point\(s\) across \d+ written/`, and assert the written count
   equals the number of `core/exam-inventory-*.json` files on disk.
2. Assert **per-track** figures in each track's own test file, so an author edits
   only their own track's line and a conflict means a genuine overlap.
3. Keep an absolute assertion but derive the expectation by summing the
   per-inventory uncovered counts, so it self-updates and still catches an
   aggregation bug.

(1) + (2) together look best: shape and inventory-count are cheap and genuinely
independent, and per-track figures keep the diagnostic value without serialising
every author behind one line.

## Do not

Do not simply delete the assertion. It has caught real aggregation changes, and a
green suite cannot distinguish an honest re-pin from a loosened one.
