## 0.1.0

- Initial C# property-based testing engine (VER05g), a port of the Java engine (VER05e)
  that conforms to the shared `pbt-v1` corpus with exact outputs, minimals and shrink steps.
- `Prng`: `SplitMix64` on `ulong` (`Bounded` with the unsigned rejection threshold, `BoundedBig`
  for spans above 2^64), `Fnv1a64`, `SubSeed`.
- `Pbt`: generator algebra (`Gens`), integrated shrinking under one global attempt budget
  (`Shrink`), the honest `Runner` with CI redaction and the closed failure vocabulary, and
  the `Corpus` conformance runner.
- `CorpusJson`: strict JSON parser (depth 64, 8 MiB, 200,000 nodes, duplicate-key refusal, strict
  UTF-8), strict value decoding, total `Canon` witness rendering.
- `Tfv`: strict TFV v1 validator backing the `corpus_reject` cases.
- Tests: whole-corpus replay, per-op corruption negative controls, engine sabotage hooks, pinned
  rejection-branch and `2^63+1` vectors, one regression test per hardening item; coverage gate 95%.
