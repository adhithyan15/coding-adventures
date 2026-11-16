## Unreleased

- Third non-Rust Warrant evidence lane (VER04i, WD-103). The package now runs
  the language-neutral `warrant-corpus-v1` corpus that the Rust compiler emits
  from `code/specs/ct-compare.warrant`, through `evidence.bindings` and the
  shared `go/warrant-corpus` runner. No library source changed.
- `evidence/test-manifest.json`, written by a real run: 12 examples, 9
  exhaustive suites (572,109 enumerated cases, domains `Bytes[347]`, `Bytes<N>`,
  `U64[377]`) and 8 negative controls, each caught by a required claim. Every row
  (ids, counts, calls, input and expected digests, `caught_by`) equals the Python
  and TypeScript lanes'. Exhaustive means exhaustive over those finite domains,
  not "all inputs".
- `evidence/domain-labels.json` labels the suites `full` (65,537 cases) or
  `bounded-set` (506,572 cases), citing backlog LP-012.
- `cmd/warrant-evidence` is the runner entry. Go cannot import a package by name
  at run time, so it compiles in the registry of adapters; the only two
  translations are the Warrant `Bool` choice into the `int` bit `CTSelectBytes`
  takes, and the library's length-mismatch panic into the `ErrLengthMismatch`
  sentinel (any other panic is re-raised and recorded as `unexpected-error`).
- `BUILD` and `BUILD_windows` (new) regenerate the manifest in memory and fail if
  the committed file drifted (`check`), prove each negative control makes the
  runner exit non-zero (`sabotage`), validate the manifest's structure
  (`validate-manifest`) and run every fault by hand, requiring a `not ok` line.
- The `go.mod` now requires the (test-time) `go/warrant-corpus` runner through a
  local `replace`, which is a build-graph edge.
