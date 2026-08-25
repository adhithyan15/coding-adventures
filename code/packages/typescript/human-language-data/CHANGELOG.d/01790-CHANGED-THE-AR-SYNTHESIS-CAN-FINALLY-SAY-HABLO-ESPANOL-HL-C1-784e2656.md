### Changed - the -ar synthesis can finally say "hablo espanol" (HL-C101)

- `hablar` was taught at chapter 15 but `espanol` sat at 22, so the synthesis
  chapter between them had nothing for the verb to take. Its exchange asked a
  bare "**Hablas?**" -- a workaround for a missing noun, not a sentence anyone
  says.
- `espanol` and the first built sentence now precede the synthesis. The exchange
  reads "**Hablas** espanol?" / "Si, **hablo** espanol", which is the most
  useful thing the whole -ar arc affords.
- The first attempt moved `espanol` alone and broke validation:
  `ES-C06-hablo-espanol` uses `trabajar` and `estudiar` as its worked examples,
  so the run had to move together. Final order is `-ar` review -> `trabajar` ->
  `estudiar` -> `espanol` -> *hablo espanol* -> **synthesis**, which is a better
  ramp than the original: the synthesis now exercises everything before it
  rather than only the three cells.
- Spanish 78 -> **79 chapters**. Fully drivable chapters **360 -> 361**;
  R2 reinforcement misses **1825 -> 1824**.

