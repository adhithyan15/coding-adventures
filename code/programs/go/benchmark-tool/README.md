# benchmark-tool

`benchmark-tool` is the first implementation slice of the repository-wide
benchmarking system.

It is intentionally language-neutral: benchmark subjects are commands, so the
same tool can compare Rust, Go, Python, Ruby, Perl, or any other executable
runtime as long as the subject can be built and invoked from a shell.

## Current Scope

Phase one supports:

- validating benchmark manifests
- parsing the CLI through the repository's declarative Go `cli-builder`
- capturing basic environment metadata
- running command-style workloads with warmup and measured trials
- writing `samples.jsonl`, `trials.jsonl`, `summary.json`, `environment.json`,
  and `report.md`
- regenerating reports from an existing result directory
- comparing two result directories at a high level

The TCP / RESP load generator described by `code/specs/benchmarking-tools.md`
is the next layer. This tool already validates those manifests, but it only
executes workloads whose driver is `command`.

## Usage

The CLI spec is embedded in the binary, so local runs do not depend on a JSON
file sitting next to the executable.
Manifest `working_directory` values are resolved relative to the manifest's git
repository root when available, which lets you run the binary from your shell
without first `cd`-ing to the repository root.

```bash
go build -o benchmark-tool .
./benchmark-tool doctor
./benchmark-tool validate ../../../benchmarks/examples/command/benchmark.toml
./benchmark-tool run ../../../benchmarks/examples/command/benchmark.toml --out /tmp/bench-result
./benchmark-tool report /tmp/bench-result
./benchmark-tool compare /tmp/baseline /tmp/candidate
```

Flags use GNU-style parsing through `cli-builder`, so both of these are valid:

```bash
./benchmark-tool run ../../../benchmarks/examples/command/benchmark.toml --out /tmp/bench-result
./benchmark-tool run --out /tmp/bench-result ../../../benchmarks/examples/command/benchmark.toml
```

## How It Fits

Correctness CI tells us whether code works. `benchmark-tool` starts building the
separate discipline for answering whether code got meaningfully faster or
slower without confusing protocol mistakes, debug builds, machine noise, or
single-number vibes for evidence.
