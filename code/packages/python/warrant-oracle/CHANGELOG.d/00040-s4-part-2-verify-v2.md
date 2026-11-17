## 0.2.0

- `verify-v2 <corpus-dir> --warrant P --profile P [--bindings P]` (VER04r section 10.3, slice
  S4 part 2): the oracle derives a whole `warrant-corpus-v2` fixture from the `.warrant`
  source and `corpus.profile` and compares it with `corpus.json`, `chunks/*.txt` and
  `cases.tfv`. New modules `corpus_v2` (derivation and the canonical writer), `faults` (the
  section 8.2 registry by signature, the witness search, control rows, `required_claims`,
  `blind_claims`, `slim_witness`, witness chunks), `tfv` (the oracle's own TFV v1 writer) and
  `verify_v2` (strict file reads, type-strict closed comparison grouped as lowering,
  enumeration, faults, controls, header, files, bytes and drift). Fault rows of a level the
  oracle lacks (`ir.fixed`) are compared as opaque data and their witnesses are `deferred`.
- Gaps closed: `corpus.profile` strictness (repeated `fault`/`slim_chunk` lines, empty
  token, `budget 0`, `slim_stride 0`); `concat` evaluated on the pure trace; v0 rules 2 and 3
  of section 3.3 including through `model_call`; IR depth 64 levels (was 65); the
  `slim_stride` clamp per suite; operation refinements lowered into the `ops` rows' `admit`.
- Tests: `tests/test_verify_v2.py`, `tests/test_controls_v2.py`, `tests/test_gaps_s4b.py`; the
  synthetic fixture `tests/fixtures/synth-v2` (a corpus produced by the Rust generator).
  `BUILD` and `BUILD_windows` run `verify-v2` on it. Tests that wrapped `U8` in a v0 document
  now declare a v1 document (the v0 guard refuses them).
