## 0.4.3 - stacked filters are charged (VER05e section 4.1)

- **Security fix.** Every `filter` sampler retry is charged whenever a budget exists: an
  attempt in a nested check, one element of work at a top-level check root. Stacked
  top-level filters can no longer multiply uncharged retries (up to 10^4 per level). A refusal
  throws `ExhaustedException`, so the check reports property-exhausted, never a pass.
  Ported from the C# review; pbt-v1 unchanged.
