# Microbenchmark delivery backlog

Land paired microbenchmark tooling across all 22 package lanes. Start with the
same indexed-array workload in each lane so collection, validation, and reporting
can be compared against one documented experiment. [VER07b contract](VER07b-polyglot-microbenchmark-comparisons.md)
Complete the all-22-lane rollout before returning to CPU simulator completion.

## Priorities

| ID | Priority | Deliverable | Dependency | Status |
|---|---|---|---|---|
| MB00 | P0 | Establish shared pairing and verdict semantics and the portable fixture. Land reusable Go and Python `rank-statistics` packages with independent exact validation and fuzz coverage. | None | Merged in [#20004](https://github.com/adhithyan15/coding-adventures/pull/20004) |
| MB01 | P0 | Go and Python benchmark reference libraries and first indexed-array drivers. Import the independently tested statistics libraries. | MB00 | Merged in [#20040](https://github.com/adhithyan15/coding-adventures/pull/20040) |
| MB03 | P1 | Reference fixture registration across build lanes. Make shared-fixture changes select all consumers and verify that packaged test sources can find the fixture where applicable. | MB01 | Implemented locally; awaiting review |
| MB02 | P1 | Language-specific rank-statistics foundations and, separately, collector harnesses/drivers. | MB01 | TS rank-only sub-slice selected; Rust/Ruby/C++ and all collectors remain separate follow-ups. |
| MB04 | P1 | Native C and Swift harnesses and array drivers using the repository's compiler and test conventions. | MB01 | Open |
| MB05 | P1 | C# and F# harnesses and array drivers. Reuse runtime statistics helpers where the public APIs remain idiomatic. | MB01 | Open |
| MB06 | P1 | Java and Kotlin harnesses and array drivers. Reuse JVM helpers where practical. | MB01 | Open |
| MB07 | P1 | Dart, Elixir, Lua, and Perl harnesses and array drivers. Use distinct PRs when toolchains or review scope warrant it. | MB01 | Open |
| MB08 | P1 | Haskell and OCaml harnesses and array drivers. | MB01 | Open |
| MB09 | P1 | Mosaic, Starlark, Twig, and WASM benchmark entry points. Verify the execution boundary and time guest batches using a monotonic host clock where appropriate. | MB01 | Open |
| MB10 | P2 | Connect paired observations to the existing benchmark runner and VER07 evidence workflows. Exercise lossless export and replay. | MB01, MB02 | Open |
| MB11 | P2 | Reproducible experiment manifest and walkthrough covering indexed traversal and growth policy. Record compiler/runtime, host, configuration, and raw samples. | MB01, MB10 | Open |

## Delivery evidence

### Selected sub-slice: MB02-TS-RANK

Implement the pure `@coding-adventures/rank-statistics` TypeScript package
specified in [VER07c](VER07c-rank-statistics.md#typescript-package-profile).
This sub-slice covers median and exact median confidence intervals, an
independent bounded oracle, deterministic properties and edge vectors,
canonical fixture replay and input registration, package/build metadata, and an
offline installed-tarball consumer. It does not include a benchmark driver,
clock, measurement collector, verdict policy, Warrant dependency or registry
publication. The benchmark-only package profile remains subject to its own
collector and approval requirements; selecting this pure library neither
inherits nor grants that approval. Reconcile later collector integration only
through the normal main/DAG review after this package merges.

- MB00 merged on November 24, 2026 as `e7ba4e7a0`.
  [Ubuntu and Windows CI](https://github.com/adhithyan15/coding-adventures/actions/runs/45541548277)
  passed after isolating the Python build tools in a package-local environment.
  Validation covered the exact sign-count oracle, Go race tests and fuzz targets,
  and 28 Python tests with 100% statement coverage. The Python source distribution
  includes the shared fixture.
- MB01 delivers the Go and Python reference collectors and indexed-array drivers.
  MB03 moves ahead of MB02 so fixture-only edits will select all existing consumers
  before additional lanes depend on the fixture. MB03 will also verify fixture access
  from packaged test sources.
- MB03 registers the canonical fixture as an exact repository input for the four
  current Go/Python consumers and verifies both language-filtered selection and
  Kiln graph export. Go package tests, race/coverage suites, both fuzz targets per
  package, vet, and builds passed. Python 3.13 suites passed (69 microbenchmark,
  28 rank-statistics); Ruff passed. Both Python sdists were built, extracted, and
  checked for byte-identical fixture access before their extracted test suites
  passed (69 and 28 tests respectively). Focused build-tool registration/export
  tests and `go vet ./...` passed. The broad build-tool suite was attempted; its
  Windows executor tests fail because the nested `cmd.exe` environment cannot
  resolve `ping`, even though the executable is present on the host; the full
  suite is not recorded as passing.

### MB03 fixture-registration contract

The canonical paired-results fixture is
`code/specs/fixtures/microbenchmark-pairs-v1.json`. A change to that exact path
must select the four existing consumers through the Go build tool's
repository-input mechanism: `go/microbenchmark`, `go/rank-statistics`,
`python/microbenchmark`, and `python/rank-statistics`. The registration must
also publish the same exact input to the build-tool graph export so fixture
changes cannot reuse stale consumer results. Deletions and rename-old paths
still select consumers; language-filtered selection selects only consumers in
that lane; unregistered neighboring paths select none; a selected consumer
missing from package discovery is an error.

The Go package tests must read a package-local fixture copy in packaged/module
contexts and compare it byte-for-byte with the canonical fixture when run in
the repository. Python source distributions must include the canonical bytes
at the tests' existing package-local fallback path. Keep runtime wheels free of
test fixtures. MB03 does not add consumers for future language lanes; each new
lane adds its reader and registration with its benchmark package.

## Completion criteria

- Each lane exposes a callable benchmark API, or a host entry point for a guest
  runtime, and runs the shared fixture and array workload.
- Portable cases cover every verdict and exact decision boundaries. New
  statistical code lives in its own package or extends an existing statistics
  package. Validate the interval calculation against an independent exact
  reference over bounded inputs, including ties and confidence cutoffs.
- Each tool has deterministic unit and integration tests, repeatable fuzzing or
  property tests, and appropriate platform validation. Exercise valid and
  invalid inputs, workload failures, timing failures, and report replay.
  Collection tests use controlled clocks. Wall-clock timing is advisory in CI.
- Smoke each native lane's actual driver and verify it emits valid raw pairs with
  positive measured durations that can be reanalyzed and replayed; do not impose
  a speed threshold. Guest-runtime lanes record the host/guest boundary overhead
  alongside their timing boundary.
- Each work item records its PR, test evidence, and merge state. Queue order is
  reconsidered after a merge or material discovery. An unresolved correctness
  or CI issue on the active PR takes priority over opening another PR.
- Where a port requires a compiler or platform unavailable locally, record the
  CI job or other execution evidence before marking the port complete.
