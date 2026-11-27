# microbenchmark (Haskell)

Runs paired in-process batches, verifies checksums, calibrates a common batch
size, and exports raw observations for strict replay. The native CLI compares
indexed reads with list traversal over one prepared 4,096-element workload.
It uses GHC's monotonic nanosecond counter, never wall time or CPU time.

```powershell
cabal run array-microbenchmark -- --pairs 40
cabal run array-microbenchmark -- --replay report.json
```

The library depends on the separately publishable `rank-statistics` package.
The command's 4 MiB report parser is backed by Aeson/Scientific and rejects
duplicate JSON keys before object decoding. Replay preserves signed 64-bit
integer tokens. Timings are advisory, with no speed threshold.

Run `cabal test` from this directory. The test suite exercises fake-clock
calibration, scheduling and failure paths; CLI acceptance should run the
native command for 40 pairs and replay its JSON.

Benchmark analysis accepts at most 10,000 pairs and inspects no more than
10,001 list cells before refusing excess input. The reusable rank-statistics
API remains uncapped. The replay command reads no more than 4 MiB plus one byte
before JSON parsing.
