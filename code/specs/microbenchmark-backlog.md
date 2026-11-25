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
| MB02 | P1 | Rust, TypeScript, Ruby, and C++ harnesses and array drivers. Evaluate reuse of Rust `statistics-core`, add reusable statistics packages where needed, and replay the same fixture. | MB01, MB03 | Reviewed locally; [#20073](https://github.com/adhithyan15/coding-adventures/pull/20073) awaiting current-head CI |
| MB04 | P1 | Native C and Swift harnesses and array drivers using the repository's compiler and test conventions. | MB01 | Reviewed locally; draft [#20080](https://github.com/adhithyan15/coding-adventures/pull/20080) awaits parent integration |
| MB05 | P1 | C# and F# harnesses and array drivers. Reuse runtime statistics helpers where the public APIs remain idiomatic. | MB01 | Implemented and locally validated; review corrections completed |
| MB06 | P1 | Java and Kotlin harnesses and array drivers. Reuse JVM helpers where practical. | MB01 | JVM Linux BUILD portability repair implemented; Windows BUILD validated locally; Linux validation awaits CI |
| MB07 | P1 | Dart, Elixir, Lua, and Perl harnesses and array drivers. Use distinct PRs when toolchains or review scope warrant it. | MB01 | Implementation in progress |
| MB08 | P1 | Haskell and OCaml harnesses and array drivers. | MB01 | Spec committed; implementation in progress |
| MB09 | P1 | Mosaic, Starlark, Twig, and WASM benchmark entry points. Verify the execution boundary and time guest batches using a monotonic host clock where appropriate. | MB01 | Open |
| MB10 | P2 | Connect paired observations to the existing benchmark runner and VER07 evidence workflows. Exercise lossless export and replay. | MB01, MB02 | Open |
| MB11 | P2 | Reproducible experiment manifest and walkthrough covering indexed traversal and growth policy. Record compiler/runtime, host, configuration, and raw samples. | MB01, MB10 | Open |
| MB12 | P1 | Align the full JSON report readers on an explicit unique-member input profile, with escaped-key duplicate, lossless-integer, malformed-text, and metadata conformance cases. Keep map-only analysis APIs distinct from raw JSON ingress. | MB01, MB02, MB04, MB10 | Swift duplicate-aware replay in progress; remaining reader profiles pending |
| MB-WIN-RUBY | P0 | Keep both Ruby microbenchmark Windows BUILD fronts aligned with the repository StandardRB line-ending contract. | MB02 | Fixed on the current MB02 branch; CI recheck pending |

## Delivery evidence

- MB05 Windows consumer follow-up: passing a local package feed and a public
  NuGet URL as repeated `dotnet restore -s` arguments resolved the URL as a
  relative path on Windows PowerShell 5.1/.NET 9. The C# and F# verifiers now
  write a temporary `NuGet.Config` with package-source mapping, sending only
  `CodingAdventures.*` packages to that feed and other dependencies to nuget.org.
  The scripts use an isolated package cache and guard recursive cleanup to the
  canonical GUID child of the temp directory. Both installed consumers pass
  locally; Linux CI remains the cross-platform validation.

- MB06 follow-up: the Unix-like Java and Kotlin `BUILD` files had invoked
  PowerShell for installed-consumer checks, so the benchmark-tool's POSIX shell
  reported `powershell: not found` after Gradle succeeded. The package `BUILD`
  fronts now call POSIX helpers; `BUILD_windows` retains the existing PowerShell
  checks. Gradle tests, coverage gates, `installDist`, POM generation, and the
  installed consumer with 40-pair replay passed locally on Windows. This host
  has no Linux runtime, so end-to-end Unix-like verification remains pending CI.

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

### MB04 integration contract

The C and Swift ports add four actual canonical-fixture consumers to the
twelve already implemented consumers. Register `c/microbenchmark`,
`c/rank-statistics`, `swift/microbenchmark`, and `swift/rank-statistics` in
selection, graph export, and fallback hash dispatch. Test exact-path filtering,
each missing consumer, and cache invalidation on a fixture-only bytes change.

C CLI validation covers unsigned-width negative wrapping, malformed UTF-8,
and buffered output failures. Both timed array paths read the same prepared
data; matching checksums alone do not establish matching work. Swift replay
must preserve Int64 tokens and reject duplicate member names before Codable
decoding can collapse them. Each lane needs a real native report and replay;
the integration expands the cross-language report checks.

The C lane's native acceptance also verifies test-link and installed-artifact
portability: POSIX rank-statistics tests pass `-lm` after source inputs, while
the installed-consumer helper builds, installs, and runs dependencies under one
explicit CMake configuration. The library target does not inherit this
test-only math link flag.

On Windows CI, when Swift setup contributes another Clang directory after the
Visual Studio bootstrap, re-promote the already selected Visual Studio
`clang-cl` directory and verify it still meets the active MSVC STL minimum.

Swift statistics and collection libraries target 95% source line coverage and
the executable targets 80%, with actual instrumented paths reported separately.
The external SwiftPM consumer uses local package paths. A sibling path dependency
does not establish independent registry release readiness; release resolution
remains an explicit follow-up before publication.

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
