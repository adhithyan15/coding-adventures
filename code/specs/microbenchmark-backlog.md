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
| MB03 | P1 | Reference fixture registration across build lanes. Make shared-fixture changes select all consumers and verify that packaged test sources can find the fixture where applicable. | MB01 | Merged in [#20064](https://github.com/adhithyan15/coding-adventures/pull/20064) |
| MB02 | P1 | Rust, TypeScript, Ruby, and C++ harnesses and array drivers. Evaluate reuse of Rust `statistics-core`, add reusable statistics packages where needed, and replay the same fixture. | MB01, MB03 | [#20073](https://github.com/adhithyan15/coding-adventures/pull/20073) current-head CI blocked by Rust capability manifests: rank-statistics is schema-v1; microbenchmark awaits signed approval for CLI time/stdout capabilities before its package-wide profile can be declared. |
| MB04 | P1 | Native C and Swift harnesses and array drivers using the repository's compiler and test conventions. | MB01 | Reviewed locally; draft [#20080](https://github.com/adhithyan15/coding-adventures/pull/20080) awaits parent integration |
| MB05 | P1 | C# and F# harnesses and array drivers. Reuse runtime statistics helpers where the public APIs remain idiomatic. | MB01 | Implemented and locally validated; review corrections completed |
| MB06 | P1 | Java and Kotlin harnesses and array drivers. Reuse JVM helpers where practical. | MB01 | Implemented and locally validated; wide-cutoff tests added |
| MB07 | P1 | Dart, Elixir, Lua, and Perl harnesses and array drivers. Use distinct PRs when toolchains or review scope warrant it. | MB01 | Implementation in progress |
| MB08 | P1 | Haskell and OCaml harnesses and array drivers. | MB01 | Spec committed; implementation in progress |
| MB09 | P1 | Mosaic, Starlark, Twig, and WASM benchmark entry points. Verify the execution boundary and time guest batches using a monotonic host clock where appropriate. | MB01 | Open |
| MB10 | P2 | Connect paired observations to the existing benchmark runner and VER07 evidence workflows. Exercise lossless export and replay. | MB01, MB02 | Open |
| MB11 | P2 | Reproducible experiment manifest and walkthrough covering indexed traversal and growth policy. Record compiler/runtime, host, configuration, and raw samples. | MB01, MB10 | Open |
| MB-WIN-RUBY | P0 | Keep both Ruby microbenchmark Windows BUILD fronts aligned with the repository StandardRB line-ending contract. | MB02 | Fixed on the current MB02 branch; CI recheck pending |

## Delivery evidence

- MB00 merged on November 24, 2026 as `e7ba4e7a0`.
  [Ubuntu and Windows CI](https://github.com/adhithyan15/coding-adventures/actions/runs/45541548277)
  passed after isolating the Python build tools in a package-local environment.
  Validation covered the exact sign-count oracle, Go race tests and fuzz targets,
  and 28 Python tests with 100% statement coverage. The Python source distribution
  includes the shared fixture.
- MB01 merged on November 24, 2026 as `62f1f9ff`. The Go and Python
  collectors and indexed-array drivers passed
  [Ubuntu and Windows CI](https://github.com/adhithyan15/coding-adventures/actions/runs/45550346426).
  The Windows performance-counter follow-up
  [#20061](https://github.com/adhithyan15/coding-adventures/pull/20061) merged
  on November 25 as `82235dcd` after
  [CI passed](https://github.com/adhithyan15/coding-adventures/actions/runs/45573731687).
  Native-driver smoke checks now complement controlled-clock tests so each
  lane verifies positive durations at its default calibration settings.
- MB03 merged on November 25, 2026 as `6739ca23`.
  [CI passed](https://github.com/adhithyan15/coding-adventures/actions/runs/45574566075).
  The planner selects the four original consumers when the fixture changes,
  and both Kiln export and legacy fallback cache keys include its bytes.
  Standalone Go sources and extracted Python source distributions replayed
  the fixture without the repository fixture directory.
- MB02 is the active item. Its four native ports extend the fixture
  registration and replay checks. MB04 remains queued behind MB02.
- Rust `rank-statistics` uses only caller-provided in-memory values and has an
  explicit empty schema-v1 profile. The Rust `microbenchmark` package also ships
  a CLI that reads the host clock and writes stdout. Its package-root profile
  therefore needs `time.read` and `stdout.write` declarations with scope settled
  by capability review; schema policy requires
  hardware-key-signed approval for non-empty profiles. No matching approval is
  present in the repository. Do not declare the package empty or add unsigned
  grants; resolve the approval before the [full-schema CI gate](https://github.com/adhithyan15/coding-adventures/actions/runs/45807004828/job/139875431748) can pass.
- The [Windows Venture acceptance step](https://github.com/adhithyan15/coding-adventures/actions/runs/45807004828/job/139875432412) in CI failed to resolve
  `vitest` after npm installed 118 packages. The preceding Windows run passed
  that same step with the same install count. The available logs do not
  establish the reason for the missing command. Recheck the step on the next
  CI run before changing Venture or shared toolchain setup.

- MB03 and the Windows native clock repair [#20061](https://github.com/adhithyan15/coding-adventures/pull/20061)
  merged with their required CI gates passing on November 25, 2026. The local
  executor lookup problem was traced to an oversized inherited Windows PATH;
  both previously failing timeout tests passed with a short child PATH. This
  resolves those two failures, without claiming a complete local build-tool run.

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

### MB02 integration contract

Each of the four ports adds two real fixture consumers: a standalone statistics
package and a paired benchmark package. Rust’s existing `statistics-core`
uses R vector semantics. The Rust harness uses its standalone
`rank-statistics` package for exact finite-sample interval cutoffs.
Register the ports’ exact discovery names
alongside the existing Go/Python consumers (twelve total), including Ruby's
`ruby/rank_statistics` directory spelling. Fixture-only edits must select all
twelve, language filters must select only that lane, and graph export must name
the same canonical file for each consumer. Do not register unimplemented lanes.
The legacy hash dispatcher must include the same file; a fixture-only bytes
change must alter each consumer's cache key without touching package sources.

Validate installed artifacts as well as source trees. Runtime metadata must
describe the toolchain that built the executable; the installed Rust executable
must not require a compiler on its runtime PATH. Collect forty positive native
pairs per lane and replay the raw reports across lanes without imposing a speed
threshold. Keep coverage, toolchain limitations, and publication status explicit.

The C++ standalone rank-statistics profile limits a sample to 10,000 values so
its private fixed-factor integer arithmetic has a documented bound. The shared
paired-report contract already limits reports to 10,000 pairs; this profile does
not change any portable benchmark fixture or report limit.

Local integration validation replayed six native forty-pair reports (Go,
Python, Rust, TypeScript, Ruby, C++) through all six implementations: all 36
combinations reproduced their recorded analyses. These timings came from a
shared development host and establish replay compatibility, not performance
rankings between languages. The Go check calls the analysis API directly;
the other five checks exercise each lane's full report replay reader.

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
