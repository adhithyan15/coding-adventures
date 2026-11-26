## 0.3.1 - shrink budget review fixes (VER05e section 4.1)

- **Security fix.** A custom `Shrinker` can now join the shrink budget: the public
  entry points (`Gen.shrink`, `shrinkList`, `shrinkBytes`, `Runner.check`) isolate
  only outside any shrink or when called from inside a property (the root budget
  tracks a property-depth counter); from shrinker context they join. A custom
  shrinker calling `Gen.shrink` per pass no longer multiplies work.
- **Fix.** A budget records when its owning scope closes; a child thread that
  outlives its scope no longer keeps the spent budget and shrinks normally.
- **Fix.** `ShrinkBudget.close` restores the ambient budget first, in a finally.
- **Security fix.** Byte shrinking charges `2 * size` per attempt (the
  list-to-array conversion of each candidate was uncharged).
- Documented: once the budget is spent the returned list/array may alias the input.
