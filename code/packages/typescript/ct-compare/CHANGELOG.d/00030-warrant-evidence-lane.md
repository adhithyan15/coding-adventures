## Unreleased

- Second non-Rust Warrant evidence lane (VER04i, WD-102). The package now runs
  the language-neutral `warrant-corpus-v1` corpus that the Rust compiler emits
  from `code/specs/ct-compare.warrant`, through `evidence.bindings`, with the
  shared `@coding-adventures/warrant-corpus` runner. No library source changed.
- `evidence/test-manifest.json` is the lane manifest written by a real run:
  12 examples, 9 exhaustive suites (572,109 enumerated cases, domains
  `Bytes[347]`, `Bytes<N>`, `U64[377]`) and 8 negative controls, each caught by a
  required claim. Every row equals the Python lane's. Exhaustive means
  exhaustive over those finite domains, not "all inputs".
- `evidence/domain-labels.json` labels the suites `full` (65,537 cases) or
  `bounded-set` (506,572 cases), citing backlog LP-012.
- `BUILD` and `BUILD_windows` (new) regenerate the manifest in memory and fail if
  the committed file drifted (`check`), prove each negative control makes the
  runner print `not ok` and exit non-zero (`sabotage`), and validate the
  manifest's structure against the corpus (`validate-manifest`).
- `tests/warrant-evidence.ts` is the lane's runner entry; `tests/warrant-binding.test.ts`
  checks the binding resolves against the real exports.
- Dev dependency on the (test-only) `@coding-adventures/warrant-corpus` runner,
  declared as `file:../warrant-corpus` so it is a build-graph edge.
