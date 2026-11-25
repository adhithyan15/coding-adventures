## 0.1.1 - security review hardening

### Fixed
- CWE-400: `gen_bytes` and `gen_bounded` corpus cases are bounded by a total
  generation budget (count x width <= 2^20), not only by per-field caps, and a
  case whose `outputs` length differs from `count` is refused before generating.
  `GenBytes` enforces the same total.
- Problem strings and `tfv` errors clip every fixture-derived fragment, so a
  hostile fixture cannot produce megabyte-sized reports.
- `decode` and `sameValue` are depth-capped at 64, so a hand-built deep tree
  passed straight to `EvaluateCorpus` is a reported problem, not a stack overflow.

### Documented
- VER05d: the CI `repro-fnv` digest is a correlation id, not a secret; it is
  brute-forceable over small witness spaces.
