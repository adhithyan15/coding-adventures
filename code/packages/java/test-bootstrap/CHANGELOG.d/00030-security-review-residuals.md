## 0.1.2

- **Security fix (CWE-400).** The shrink attempt budget is global per shrink:
  `tuple2`/`tuple3` share one `16 * maxSteps + 64` budget across all passes and
  components (it was a fresh budget per pass, quadratic in `maxSteps`), and
  `Runner.check` applies one per falsified case.
- Sequence shrinking copies a rejected deletion candidate once instead of twice.
- **Security fix.** `Canon` and the `Runner` fallbacks (and the untrusted-shrinker
  guard) also catch `AssertionError` and `LinkageError`, so a hostile `toString`
  or iterator cannot make `Runner.check` throw. `OutOfMemoryError` and other
  `VirtualMachineError`s are still not caught.
- **Security fix (CWE-400).** Public `Gens.genBounded` enforces the
  `count x width(hi - lo)` byte budget itself.
