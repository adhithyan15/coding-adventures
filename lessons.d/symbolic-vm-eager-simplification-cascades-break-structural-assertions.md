---
category: Compiler / VM / language pipeline
---

# Symbolic-VM eager-simplification cascades break structural assertions

When porting algebraic-rule phases between languages, a single new rule fires on every re-eval of every matching subterm — including transient subterms produced by *other* handlers. PR #3468's Phase 30 rule `exp(n·log(x)) → x^n` turned `D(x^x, x)` from `exp(x·log(x))·(log(x)+1)` (the prior handler-internal intermediate) into `x^x·(log(x)+1)` because the derivative handler emits `exp(x·log(x))` internally and that node is re-evaluated. Mathematically equivalent, but structural-match tests using `toEqual` / `assert_eq!` on the old form fail. Lesson: when adding a new global rule, grep for tests that hand-write the affected intermediate form and update both shape expectations and the CHANGELOG "regression note" — don't pin the structural test to either form, expect the simplest one.
