# VER07v — Native microbenchmark fixture integration

The paired-observation fixture is an external input to every implemented
native microbenchmark reader. A change to
`code/specs/fixtures/microbenchmark-pairs-v1.json` must select the Go build
tool's exact reader set and must change each selected package's fallback cache
key. The same input paths must appear in the exported Kiln graph.

## Readers

This integration adds the twelve packages delivered by MB07 and MB08 to the
twenty-five existing fixture consumers, for thirty-seven consumers total:

- Dart: `dart/microbenchmark`, `dart/rank-statistics`.
- Elixir: `elixir/microbenchmark`, `elixir/rank_statistics`.
- Lua: `lua/microbenchmark`, `lua/rank_statistics`.
- Perl: `perl/microbenchmark`, `perl/rank-statistics`.
- Haskell: `haskell/microbenchmark`, `haskell/rank-statistics`.
- OCaml: `ocaml/microbenchmark`, `ocaml/rank-statistics`.

The already registered `go/programs/benchmark-tool` reader remains selected.
Do not add planned but unimplemented lanes.

## Build-tool contract

Register each exact discovery package name against the canonical fixture path
using the existing microbenchmark fixture matcher. Keep selection, the
`repositoryInputsForPackage` fallback dispatcher, and Kiln export aligned. A
fixture-only byte change must change each consumer's fallback hash. Exact-path
tests reject neighboring filenames and noncanonical spellings; a missing
registered consumer is a hard error; language filters select only that
language's readers. Haskell's prior empty-filter case becomes a real two-reader
case, and WASM remains the unrelated empty language filter.

## Validation and limits

The Go build-tool runs focused selection, export, and fallback-cache tests plus
`go vet`. The integrated Dart package must still pass VM and compiled-Node tests
and a native 40-positive-pair report/replay. Other native lanes retain their
own driver validation; the integration does not claim all twenty-two package
lanes are complete or merged. Timings establish that native clocks execute,
not a performance threshold.
