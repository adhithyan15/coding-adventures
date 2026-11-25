## 0.4.2 - sampling, starvation and estimate fixes (VER05e section 4.1)

- **Security fix.** The `filter` sampler charges one attempt per retry inside a nested check
  (refused: property-exhausted); `maxRetries` is capped at `Limits.MAX_FILTER_RETRIES` (10,000).
- **Fix.** A top-level check whose nested checks or generators were starved returns
  property-exhausted rather than passed; a falsified verdict is never overridden.
- **Fix.** The generation work estimate counts one level deeper (lists of lists, arrays of
  collections, strings), capped at 10,000 elements visited; hostile collections count 0.
- Non-redacted witness rendering uses the same total wrapper as the redacted path.
