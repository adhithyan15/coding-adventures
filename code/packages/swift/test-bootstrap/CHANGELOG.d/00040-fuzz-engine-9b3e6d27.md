## 0.2.0

- Swift lane of the VER06 coverage-guided fuzz engine (spec VER06l; the letter after the C lane's
  VER06j, `k` is held for C++). `Sources/TestBootstrap/Fuzz/`: the mutator algebra (`UInt64` and
  `BigInt` forms, parity flips, floor-modulo offsets, no reachable traps), `Bitmap`, `corpusAdmit`,
  `calculateEnergy` (half away from zero), `FuzzSignature` (crash signatures, the opaque TAP line,
  kind and frame normalisation), the closed `FuzzFailureIds`, `Handle`, `FuzzEngine` (executor,
  minimiser, deterministic campaign), `FuzzReport` / `CrashRecord`, and the `FuzzCorpus` runner.
- Coverage source: caller-reported edges (`Handle.edge`); a run that reports none is labelled
  `black-box-uninstrumented`. `-sanitize-coverage=trace-pc-guard` is documented as future work
  (SwiftPM refuses `unsafeFlags` in dependencies; guard ids are not stable across build modes).
- Hardening from the C#, Java, Go and C reviews, built in from the start: a thrown error is a crash
  whose kind is the error's dynamic type name (its `description` is never called); frames are a
  caller-supplied label stack (`Handle.scope`); budgets are thrown identity-checked sentinels,
  sticky and shared, so a target that swallows one is still a timeout or OOM; a forged or stale
  sentinel is the crash `stale-budget-sentinel`; a FRESH handle per execution is retired under a
  lock, so a stale `Thread` or `Task` is inert; the input is physically copied; retained bytes are
  capped at 256 MiB with counted drops; minimisation runs under a campaign-wide budget that counts
  rejected candidates; `mutateSequence` charges the bytes really copied (failed calls included) and
  the corpus runner zeroes its 2^24 aggregate on a breach; case ids are escaped in problem lines;
  no rendering shows input bytes or the seed.
- Tests: the 25 fuzz-v1 cases replay exactly with per-case corruption and sabotage controls, Go's
  820 oracle vectors (parsed from its source) through both integer paths, pinned signatures,
  energy rows including half-way ties, interest and delta constants, unequal splice splits, and
  campaign tests for dedup, sticky exhaustion, stale threads and tasks, sentinels, caps and
  minimisation budgets.
