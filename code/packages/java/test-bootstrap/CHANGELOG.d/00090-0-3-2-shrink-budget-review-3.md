## 0.3.2 - shrink budget review fixes, round three (VER05e section 4.1)

- **Security fix.** Property context is per thread (`InheritableThreadLocal<int[]>`)
  instead of a counter on the shared budget: a worker blocked inside a property of a
  parallel custom shrinker no longer makes another thread's `Gen.shrink` /
  `shrinkList` / `shrinkBytes` / `Runner.check` isolate and reopen amplification.
  A thread spawned inside a property stays in property context.
- **Fix.** The depth guard saves and restores by value with no method call in the
  `finally` (Runner included), so a StackOverflowError cannot leave it raised.
- **Fix.** `Gens.filter`'s user predicate runs in property context, so a shrink or
  `Runner.check` inside it is isolated rather than joining the outer budget.
