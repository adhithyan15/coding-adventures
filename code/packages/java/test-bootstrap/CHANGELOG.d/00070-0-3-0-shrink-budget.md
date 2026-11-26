## 0.3.0 - shrink budget ported from the C# security review (VER05e section 4.1)

- **Security fix (CWE-400).** Shrinking is now budgeted by work as well as by
  attempts. One `ShrinkBudget` per shrink charges attempts (`16 * maxSteps + 64`)
  and list elements copied (`Limits.MAX_SHRINK_WORK`, 64M). A 1 MiB `bytes`
  witness with a stalling property no longer costs tens of GB of copying.
- **Security fix.** The budget is shared by nested shrinkers (list of lists,
  tuple components, filter). It is ambient (`InheritableThreadLocal`) so threads
  created inside a shrink join it; already-running pooled threads do not (documented).
- **Security fix.** Initial copies and the `bytes`/`shrinkBytes`/`shrinkList`
  conversions are charged before they happen and return the input uncopied once
  the budget is spent, so tuple passes cannot keep copying.
- `Runner.check`, `Gen.shrink`, `Shrink.shrinkList` and `Shrink.shrinkBytes` open
  an isolated scope restored in a `finally`: a property that calls `Runner.check`
  gets the same inner verdict as a standalone run. Charge counters are thread-safe.
- Accepted moves mutate the current list in place (no second copy). pbt-v1 stays
  25/25 with exact minimals and step counts.
- Regression tests in `ShrinkBudgetTest`; sabotage (removing the work charge) fails 5 of them.
