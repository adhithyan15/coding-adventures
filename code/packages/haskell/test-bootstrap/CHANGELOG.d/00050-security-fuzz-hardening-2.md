## 0.1.4 - second security review of the fuzz engine (VER06o)

- MEDIUM: the leak cap is enforced during minimisation too: the probe and both passes ask the abort flag before every
  candidate, so a target that crashes and then masks-and-spins on candidates leaks exactly 4 workers (tested).
- LOW: `verdict` re-verification shares the leak counter (`verdictTracked`/`executorTracked`); a leaked-out report
  re-verifies nothing and re-verification stops at the cap, the rest `Unverified` (tested).
- LOW: `leakedKillWaitMs` is 500 ms; "leaked" is documented as "did not answer the kill within the wait".
- Executed candidates and triage seeds are no longer copied; only the three retention points copy. New tests pin each of
  the three copies (removing any one fails its assertion) and check admitted mutated candidates against max_input_len.
- `Fuzz.Handle.Internal` is `not-home` in haddock and states plainly that it breaks the engine's guarantees.
