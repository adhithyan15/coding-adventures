## [0.472.0] - UNRELEASED

- Extend bounded `entier(sign(...))` additive-offset proofs from ten to eleven exact units, including pathwise runtime conditionals and checked exact sums.

- Reconcile only four dedicated canonical budget/comment edits; preserve every shared folding, exp/log, alias/effect, read-once and production-limit guard.

Actual source `f630e6ad` passed 2,765 tests across 120 targets, zero failures,
one unchanged parent-only archive ignore, and strict all-target Clippy for all
20 affected packages. Compiler884 and full unfiltered LANG500 passed. Focused
runtime has 116 sources/696 actual cells on six available backends; all636 old
tuples and106 old sources stay exact. CLR is unavailable and uncounted.
Source/native-focused review19fd3975 and style-focused76b54bb9 passed; independent
full terminal review is pending. Hosted checks and publication remain separate.
Final-source Miri **DEFERRED/PENDING**, never passed.
