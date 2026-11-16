## 0.1.0

- Initial TypeScript lane runner for the `warrant-corpus-v1` evidence corpus
  (VER04i, WD-102), mirroring the Python lane (WD-101): strict corpus loader
  with its own JSON reader (duplicate keys, non-safe-integer numbers and
  prototype keys refused), in-lane domain enumeration with `warrant-stream-v1`
  digests cross-checked against the corpus, a model-IR interpreter that compiles
  to closures (no `eval`, no `Function`), the binding table
  (`evidence.bindings`, errors mapped by exact prototype), the ten portable
  fault wrappers, example, exhaustive and negative-control suites, the
  `warrant-lane-evidence-v1` manifest writer and drift check, the
  `domain-labels.json` sidecar (backlog LP-012), and a structural manifest
  validator.
- Bounded work: product size equals `cases + discarded`, at least one admitted
  case, per-suite and total caps, a static IR cost limit, operand range checks
  before arithmetic and depth caps.
- `U64` and `Nat` are `bigint` throughout; `Bytes` results must be an exact
  `Uint8Array` and a `fresh` result may not alias an argument.
- No runtime dependencies. Runs as TypeScript through Node's type stripping.
