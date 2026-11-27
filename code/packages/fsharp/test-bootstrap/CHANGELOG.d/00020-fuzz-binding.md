## 0.2.0

- F# binding over the C# fuzz-v1 coverage-guided fuzz engine (VER06m), in namespace
  `CodingAdventures.TestBootstrap.FSharp.Fuzzing`: `Campaign.run` and the `fuzz { }`
  builder, an F# `Handle` (`Edge`/`Tick`/`Reserve`), `FuzzOptions` (a `uint64` seed that
  never prints), class views `FuzzReport`/`CrashRecord` that never print input bytes or the
  seed, the `Mutate`, `Admission`, `Energy`, `Signature`, `Tap` and `FuzzFailureId` modules
  (one wrapper per fuzz-v1 operation).
- Replays the whole shared fuzz-v1 corpus (25/25 exact) through the F# API with its own
  runner, per-op negative controls and nine F#-layer sabotage seams.
- Pinned-value tests of every public wrapper (every mutator, admission, all energy rows, real
  crash-signature hashes, dedup, TAP, failure ids) plus differential grids against the engine.
- Campaign tests: dedup, sticky exhaustion in every swallow shape, a stale handle from a
  spawned `Task`, crashes from F# exceptions, `failwith`, `async { }` and `task { }`, and a
  redacted `%A`/`ToString` of report, crash record and options.
- Build tool: `fsharp/test-bootstrap` is registered as a fuzz-v1 corpus consumer.
