# Microbenchmark delivery backlog

Land paired microbenchmark tooling across all 22 package lanes. Start with the
same indexed-array workload in each lane so collection, validation, and reporting
can be compared against one documented experiment. [VER07b contract](VER07b-polyglot-microbenchmark-comparisons.md)

## Priorities

| ID | Priority | Deliverable | Dependency | Status |
|---|---|---|---|---|
| MB00 | P0 | Establish shared pairing and verdict semantics and the portable fixture. Land reusable Go and Python `rank-statistics` packages with independent exact validation and fuzz coverage. | None | In progress |
| MB01 | P0 | Go and Python benchmark reference libraries and first indexed-array drivers. Import the independently tested statistics libraries. | MB00 | Local prototypes |
| MB02 | P1 | Rust, TypeScript, Ruby, and C++ harnesses and array drivers. Evaluate reuse of Rust `statistics-core`, add reusable statistics packages where needed, and replay the same fixture. | MB01 | Local prototypes |
| MB03 | P1 | Reference fixture registration across build lanes. Make shared-fixture changes select all consumers and verify that packaged test sources can find the fixture where applicable. | MB01 | Open |
| MB04 | P1 | Native C and Swift harnesses and array drivers using the repository's compiler and test conventions. | MB01 | Open |
| MB05 | P1 | C# and F# harnesses and array drivers. Reuse runtime statistics helpers where the public APIs remain idiomatic. | MB01 | Open |
| MB06 | P1 | Java and Kotlin harnesses and array drivers. Reuse JVM helpers where practical. | MB01 | Open |
| MB07 | P1 | Dart, Elixir, Lua, and Perl harnesses and array drivers. Use distinct PRs when toolchains or review scope warrant it. | MB01 | Open |
| MB08 | P1 | Haskell and OCaml harnesses and array drivers. | MB01 | Open |
| MB09 | P1 | Mosaic, Starlark, Twig, and WASM benchmark entry points. Verify the execution boundary and time guest batches using a monotonic host clock where appropriate. | MB01 | Open |
| MB10 | P2 | Connect paired observations to the existing benchmark runner and VER07 evidence workflows. Exercise lossless export and replay. | MB01, MB02 | Open |
| MB11 | P2 | Reproducible experiment manifest and walkthrough covering indexed traversal and growth policy. Record compiler/runtime, host, configuration, and raw samples. | MB01, MB10 | Open |

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
- Each work item records its PR, test evidence, and merge state. Queue order is
  reconsidered after a merge or material discovery. An unresolved correctness
  or CI issue on the active PR takes priority over opening another PR.
- Where a port requires a compiler or platform unavailable locally, record the
  CI job or other execution evidence before marking the port complete.
