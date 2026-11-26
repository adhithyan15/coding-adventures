## 0.2.0

- Kotlin binding over the Java coverage-guided fuzz engine (spec VER06g,
  PT-KT-FUZZ). No second engine (WD-901); no change to `java/test-bootstrap`.
- API: `fuzz { seed; iterations; seeds(...); target { data, h -> } }`,
  `runCampaign(corpus, FuzzOptions, target)`, a Kotlin `Handle` (`edge(ULong)`,
  `edge(label)`, `tick`, `reserve`), `FuzzOptions` (seed is `ULong`, redacted in
  `toString`), read-only `Report` / `CrashRecord` views (not data classes; `toString`
  never prints input bytes or the seed; `seed`, `corpus()` and `minimalInput()`
  are explicit private accessors), `CoverageTier`, `Report.tapFailures`.
- Wrappers for every fuzz-v1 op: nine `ByteArray` mutators (`Long` and
  `BigInteger` overloads, `Endian`, `ULong` boundary values), `corpusAdmit`,
  `calculateEnergy`, `crashSignature`, `isDuplicateCrash`, `FuzzFailureId`,
  `formatTapFailure`, `tfvIsValid`.
- Behaviour notes: `TODO()` (`kotlin.NotImplementedError`) is a crash in the
  fuzz engine; Kotlin function references add a synthetic frame per call site.
- Tests: `KotlinFuzzCorpusRunner` replays all 25 fuzz-v1 cases (including the
  crash-dedup signature hashes) through the Kotlin API only, never
  `fuzz.Corpus.evaluate`; per-case corruption controls, dropped/truncated/
  duplicated cases, six new Kotlin-layer sabotage seams; API tests for the DSL,
  dedup, timeout/OOM, redaction. JaCoCo gate stays at 95% lines.
- Version bumped to 0.2.0.
