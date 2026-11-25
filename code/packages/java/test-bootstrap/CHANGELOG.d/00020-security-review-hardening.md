## 0.1.1

- **Security fix (CWE-400).** `genPrimitive`, `genBounded` and `genBytes` reject
  a `count` above `Limits.MAX_GEN_COUNT` (1,000,000); `genBytes(seed, 0, 0,
  Integer.MAX_VALUE)` used to skip its budget and exhaust the heap.
- **Security fix (CWE-400).** Sequence shrinking charges rejected attempts
  against a budget of `16 * maxSteps + 64` evaluations, so large lists no longer
  cost O(n^2) regardless of `maxSteps`. pbt-v1 minimals and step counts are
  unchanged.
- **Security fix.** `Canon.render` is now truly total (hostile iterators,
  entry sets, char sequences and stack overflows render as `<unrenderable>`), and
  `Runner` falls back to a constant digest preimage, so redacted `check` always
  returns a verdict.
- **Security fix (CWE-400).** `StrictJson` caps total values per document at
  200,000.
- Documented that the redacted digest does not hide low-entropy witnesses and
  that there is no wall-clock timeout.
