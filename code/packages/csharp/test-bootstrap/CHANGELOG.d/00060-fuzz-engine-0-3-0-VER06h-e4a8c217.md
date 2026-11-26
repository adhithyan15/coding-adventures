## 0.3.0

- Adds the coverage-guided fuzz engine (VER06h): namespace `CodingAdventures.TestBootstrap.Fuzz`, a
  port of the Java lane (VER06f) conforming to the shared `fuzz-v1` corpus (25/25) and to the 820
  reference vectors of the Go lane through both the `ulong` and `BigInteger` entry points.
- `Mutate` (algebra with parity flips, `MutateSequence` capped by steps, input size and actual copy
  work), `Bitmap` (65,536-edge map, `CorpusAdmit`, `CalculateEnergy` with half-up rounding),
  `FuzzSignature` (crash signature = low 32 bits of FNV-1a, opaque TAP lines), `FuzzEngine` (executor,
  minimiser, campaign), `FuzzCorpus` (strict corpus runner), `Handle` (caller-reported edges, sticky
  step and memory budgets).
- .NET divergences from Java: the budget sentinel is an ordinary exception (stickiness plus a post-run
  check carry the guarantee); frames come from `new StackTrace(exception)` so no hostile virtual member
  is called; `StackOverflowException` is uncatchable (documented); no `CheckForOverflow` reliance.
- Tests: corpus replay with per-case corruption, drop and truncation controls and 14 sabotage hooks;
  reference vectors; pinned energy values, signatures, interest/delta constants and unequal splice
  splits (corpus blind spots PT-F1/F7); campaign, sticky-exhaustion, hostile-exception, OOM,
  retained-bytes and minimisation-budget tests. `required_capabilities.json` stays empty.
- Version 0.3.0.
