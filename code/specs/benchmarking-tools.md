# Benchmarking Tools

## Overview

The repository needs a first-class benchmarking toolkit, not scattered one-off
scripts. The TCP runtime work made that need obvious: a result that looked like
"1000 connections take 5.7 seconds" turned out to depend heavily on how the
client read responses. That is exactly the sort of ambiguity a benchmark tool
should prevent.

The goal of this spec is to define a language-neutral benchmarking system that
can answer questions like:

- Did this branch make Mini Redis faster or slower?
- Is the Rust TCP runtime faster than a Python, Ruby, Go, or Perl server for
  the same protocol workload?
- Did a change improve throughput but hurt p99 latency?
- Is an observed difference statistically meaningful, or just noise from the
  machine, scheduler, network stack, or load generator?
- Can we reproduce the result later with enough context to trust it?

The core rule is:

> A benchmark compares distributions, not single numbers.

One timing value is a smoke test. A benchmark result is a structured collection
of samples, environment metadata, statistical summaries, and comparison rules.

---

## Why This Exists

The repo already has many correctness tools:

- package-local tests
- coverage gates
- BUILD files
- CI change detection
- security review workflows

It does not yet have an equivalent system for performance. That gap matters
because the repo increasingly contains runtime-like packages:

- `transport-platform`
- `stream-reactor`
- `tcp-runtime`
- Mini Redis
- IRC
- parsers, codecs, VM runtimes, storage engines, and compression libraries

Without a shared benchmark system, agents will keep writing ad hoc scripts that
mix together:

- server runtime cost
- client harness cost
- process startup cost
- protocol framing mistakes
- debug versus release builds
- one branch being warm in the cache while another is cold
- OS backlog limits and scheduler noise

Those scripts can be useful for local intuition, but they are not enough for
deciding architecture.

---

## Design Goals

The benchmark tools must:

- benchmark any executable command, regardless of implementation language
- compare two git refs, two binaries, or two language implementations
- preserve enough environment metadata to make results interpretable
- distinguish warmup, measurement, and cooldown phases
- collect raw samples in an append-only machine-readable format
- report confidence intervals, effect sizes, and practical thresholds
- support service benchmarks where a server is started and then loaded
- support microbenchmarks where a command emits its own samples
- make benchmark mistakes hard, especially EOF-delimited reads for framed
  protocols like RESP, IRC, HTTP, and WebSocket
- keep correctness checks near the benchmark so a faster wrong implementation
  cannot win
- produce Markdown reports that are useful in PRs

## Non-Goals

The first version should not:

- replace Criterion, pytest-benchmark, Go benchmarks, BenchmarkDotNet, or other
  language-native microbenchmark libraries
- require every language implementation to link against a shared benchmark
  library
- run noisy performance gates on every normal CI push
- hide raw samples behind only one summary number
- declare winners based only on p-values
- try to normalize fundamentally different workloads into one global score

Language-native benchmark libraries remain useful inside a package. This repo
tool sits above them and gives the monorepo one way to run, compare, and report
results.

---

## Proposed Toolset

The benchmark system should be a small family of tools and schemas.

### `benchmark-tool`

The main CLI program.

Responsibilities:

- read benchmark manifests
- prepare subjects
- run warmups and measured trials
- collect samples
- compare variants
- write reports
- validate result files

Initial location:

```text
code/programs/go/benchmark-tool/
```

Go is a good first implementation language because the repo already uses Go for
the primary build tool, process orchestration is simple, and the tool can stay
independent of the systems being benchmarked.

### `benchmark-schema`

A language-neutral schema package or spec directory.

Responsibilities:

- define result JSON / JSONL shapes
- define manifest validation rules
- define stable metric names
- define comparison verdict fields

Initial location:

```text
code/specs/schemas/benchmark-result.schema.json
code/specs/schemas/benchmark-manifest.schema.json
```

### Load Generators

Load generators are benchmark subjects' sparring partners. They generate work
and collect client-observed measurements.

Initial TCP-oriented load generator:

```text
code/programs/go/benchmark-load-tcp/
```

Responsibilities:

- open connections
- run frame-aware protocol workloads
- measure connect, write, first-byte, full-frame, and total latency
- validate responses
- record per-client and aggregate samples

The first implementation should support Mini Redis RESP workloads because the
TCP runtime currently needs that most.

### Language Adapters

Adapters are optional helpers that make language-native benchmarks emit the
repo's canonical sample format.

Examples:

- Rust helper that wraps Criterion output
- Python helper that wraps pytest-benchmark output
- Go helper that wraps `go test -bench` output
- Ruby helper that wraps benchmark-ips output

Adapters are not required for process-level benchmarks. Any command can
participate if it emits the expected JSONL samples or if `benchmark-tool`
measures it externally.

---

## Core Concepts

### Subject

A subject is the thing being benchmarked.

Examples:

- one git ref of `mini-redis`
- one executable binary
- one Python module command
- one Ruby native extension
- one Go server
- one Docker image in a future phase

Subjects are intentionally command-based:

```toml
[[subjects]]
name = "rust-mini-redis-main"
kind = "service"
checkout = "origin/main"
build = "cargo build --release"
command = "./target/release/mini-redis --host 127.0.0.1 --port {port}"
working_directory = "code/programs/rust/mini-redis"
```

That keeps the benchmark system language-neutral.

### Workload

A workload is the traffic or operation pattern applied to a subject.

Examples:

- 1000 one-shot Redis `PING` clients
- 1000 preconnected clients firing at once
- 10000 pipelined `SET` / `GET` operations over 100 connections
- 50000 parser invocations over a fixed corpus
- 1 million hash-map inserts
- 10000 compression round-trips

### Trial

A trial is one measured run of one subject under one workload.

Each trial produces:

- raw samples
- aggregate metrics
- correctness verdict
- environment metadata
- logs and exit status

### Sample

A sample is one measured observation.

For service benchmarks, a sample might be one request:

```json
{
  "sample_kind": "request",
  "subject": "rust-mini-redis-main",
  "workload": "redis-ping-1000",
  "trial": 7,
  "ok": true,
  "metrics": {
    "connect_ms": 2.31,
    "write_ms": 0.04,
    "first_byte_ms": 0.88,
    "frame_ms": 0.91,
    "total_ms": 3.42
  }
}
```

For microbenchmarks, a sample might be one iteration batch:

```json
{
  "sample_kind": "operation_batch",
  "subject": "rust-hash-map",
  "workload": "insert-1m",
  "trial": 3,
  "ok": true,
  "metrics": {
    "operations": 1000000,
    "elapsed_ms": 81.7,
    "ops_per_second": 12239902.08
  }
}
```

---

## Benchmark Manifest

The manifest should be checked into the repo near the benchmark suite. It
describes what to run, not just how to summarize it.

Recommended initial location:

```text
code/benchmarks/
```

Example:

```toml
name = "mini-redis-tcp-runtime"
description = "Mini Redis workloads used to tune tcp-runtime."

[defaults]
warmup_trials = 3
measurement_trials = 30
cooldown_ms = 250
randomize_subject_order = true
fail_fast = false

[[subjects]]
name = "current"
kind = "service"
checkout = "HEAD"
working_directory = "code/programs/rust/mini-redis"
build = "cargo build --release"
command = "./target/release/mini-redis --host 127.0.0.1 --port {port}"
ready_check = "tcp-connect"

[[subjects]]
name = "baseline"
kind = "service"
checkout = "origin/main"
working_directory = "code/programs/rust/mini-redis"
build = "cargo build --release"
command = "./target/release/mini-redis --host 127.0.0.1 --port {port}"
ready_check = "tcp-connect"

[[workloads]]
name = "redis-ping-1000"
driver = "tcp-resp"
connections = 1000
concurrency = 1000
mode = "one-shot"
request = "*1\\r\\n$4\\r\\nPING\\r\\n"
expect = "+PONG\\r\\n"
read_mode = "resp-frame"

[[workloads]]
name = "redis-preconnected-ping-1000"
driver = "tcp-resp"
connections = 1000
concurrency = 1000
mode = "preconnect-then-fire"
request = "*1\\r\\n$4\\r\\nPING\\r\\n"
expect = "+PONG\\r\\n"
read_mode = "resp-frame"

[[workloads]]
name = "redis-pipelined-set-get"
driver = "tcp-resp"
connections = 100
requests_per_connection = 100
mode = "pipeline"
read_mode = "resp-frame"
```

### C10K Hold Workloads

C10K is not a short-lived connection churn benchmark. A real C10K hold
workload proves that a server can keep 10,000 TCP sockets open at the same
time, survive a hold period, and still respond afterward.

For `driver = "tcp-resp"`, `mode = "hold"` means:

1. ramp-open `connections` client sockets with at most `concurrency` dials in
   flight
2. keep every successfully opened socket alive until all dial attempts finish
3. hold the full connected set open for `hold_ms`
4. send the declared RESP `request` on every surviving socket
5. parse and validate the declared RESP `expect` frames before closing sockets

Example:

```toml
[[workloads]]
name = "redis-c10k-hold"
driver = "tcp-resp"
mode = "hold"
connections = 10000
concurrency = 500
hold_ms = 60000
timeout_ms = 5000
request = "*1\r\n$4\r\nPING\r\n"
expect = "+PONG\r\n"
read_mode = "resp-frame"
```

The result is a capacity and liveness proof, not a throughput proof. The most
important trial metrics are:

- opened connections
- failed operations
- connect-time distribution
- total wall-clock time
- post-hold response correctness

If this workload fails, the report should make the failure mode clear enough to
separate:

- load-generator file descriptor limits
- client ephemeral port exhaustion
- server-side connection caps
- server accept-loop limits
- per-connection memory pressure
- event backend registration or polling bugs

### Manifest Rules

The tool must reject ambiguous manifests.

Rules:

- service benchmarks must declare a readiness check
- framed protocols must declare a frame-aware read mode
- EOF reads are only allowed when the manifest explicitly states that the
  protocol closes after each response
- every subject must declare a build command or state that it is prebuilt
- every workload must declare correctness expectations
- every comparison must declare the primary metric and practical threshold
- command placeholders like `{port}` must be explicit and validated

---

## Statistical Model

Benchmark reports must make two separate claims:

1. Is the observed difference statistically credible?
2. Is the observed difference large enough to matter?

Both are required. A tiny statistically significant change can be irrelevant.
A large apparent change can be untrustworthy if the samples are too noisy.

### Required Summary Statistics

For each metric, report:

- sample count
- min and max
- mean
- median
- standard deviation
- median absolute deviation
- p50, p90, p95, p99
- bootstrap confidence interval for the median
- bootstrap confidence interval for the mean

For throughput-style metrics, also report:

- total operations
- operations per second
- bytes per second when bytes are meaningful

### Required Comparison Statistics

For two variants, report:

- absolute difference
- relative difference
- paired difference when the trial order supports pairing
- bootstrap confidence interval for the relative difference
- non-parametric effect size, such as Cliff's delta
- verdict against a practical significance threshold

Recommended default:

```text
Winner only if:
  - correctness passed for both subjects
  - the bootstrap confidence interval for relative difference excludes zero
  - the median relative difference exceeds the configured practical threshold
```

Default practical thresholds:

- latency metrics: 5 percent
- throughput metrics: 5 percent
- memory metrics: 5 percent
- startup time: 10 percent

The thresholds are defaults, not laws. Each benchmark suite may choose stricter
or looser thresholds, but the report must state them.

### Outlier Policy

Default behavior:

- keep all samples
- mark extreme values
- report robust statistics
- never silently drop outliers

Allowed explicit behavior:

- drop warmup samples from measured summaries
- drop samples from failed correctness checks
- rerun a trial when the benchmark harness itself failed before reaching the
  subject, such as a load generator crash

Disallowed behavior:

- deleting slow successful samples just because they look ugly
- averaging p99 values across unrelated runs and treating that as a p99
- reporting only the best run

### Trial Ordering

When comparing two subjects, run order should avoid favoring the second subject
with warmer caches or the first subject with a quieter system.

Recommended strategies:

- randomize subject order for each trial
- support paired A/B trials when both variants can run the same workload
- support ABBA blocks for two-subject comparisons
- record actual run order in the result file

Example ABBA block:

```text
trial block 1: A B B A
trial block 2: B A A B
```

### Minimum Sample Counts

Defaults:

- smoke benchmark: 3 warmup, 5 measured trials
- local investigation: 3 warmup, 30 measured trials
- PR report: 5 warmup, 50 measured trials when runtime is reasonable
- nightly benchmark: 10 warmup, 100 measured trials

For service benchmarks where each trial contains many request samples, the
tool should report both:

- per-request distributions within each trial
- per-trial aggregate distributions across trials

The comparison verdict should prefer per-trial aggregates so one giant trial
with thousands of requests does not fake independence.

---

## Environment Capture

Every run directory must contain an `environment.json` file.

Required fields:

- timestamp
- hostname
- operating system
- kernel version
- architecture
- CPU model when available
- logical CPU count
- memory size when available
- git repository URL
- git ref and commit SHA for each subject
- dirty worktree status for each subject
- compiler or interpreter version
- build profile, such as debug or release
- relevant OS limits, such as file descriptor limit and listen backlog limit
- environment variables selected by the manifest

For TCP benchmarks, also capture when available:

- `ulimit -n`
- listen backlog setting
- OS maximum socket backlog
- ephemeral port range
- TCP TIME_WAIT / MSL related settings
- loopback interface information

The tool should not require root privileges. If a value cannot be collected
without elevated permissions, record it as unavailable.

---

## Result Directory Layout

Each benchmark run writes a self-contained directory:

```text
benchmark-results/
  2026-04-20T15-04-22Z-mini-redis-tcp-runtime/
    manifest.toml
    environment.json
    subjects/
      current/
        build.log
        server.log
      baseline/
        build.log
        server.log
    samples.jsonl
    trials.jsonl
    summary.json
    comparison.json
    report.md
```

### `samples.jsonl`

Append-only per-sample observations.

### `trials.jsonl`

One row per subject/workload/trial with aggregate metrics.

### `summary.json`

Per-subject, per-workload statistical summaries.

### `comparison.json`

Pairwise comparisons and verdicts.

### `report.md`

Human-readable report for PRs and design discussions.

---

## CLI Shape

### `benchmark-tool doctor`

Checks whether the host can run stable-ish benchmarks.

Examples:

```bash
benchmark-tool doctor
benchmark-tool doctor --tcp
```

Checks:

- required toolchains are available
- output directory is writable
- CPU count can be detected
- high-resolution timer is available
- file descriptor limit is high enough for requested TCP workloads
- requested ports are available

### `benchmark-tool run`

Runs a manifest.

```bash
benchmark-tool run code/benchmarks/mini-redis/benchmark.toml \
  --out benchmark-results/local-mini-redis
```

Useful flags:

```text
--subjects current,baseline
--workloads redis-ping-1000
--trials 30
--warmup 3
--seed 12345
--reuse-builds
--fail-fast
```

### `benchmark-tool compare`

Compares two existing result directories or two subjects inside one result.

```bash
benchmark-tool compare \
  benchmark-results/before \
  benchmark-results/after \
  --metric frame_ms \
  --threshold 0.05
```

### `benchmark-tool report`

Regenerates Markdown from raw samples and summaries.

```bash
benchmark-tool report benchmark-results/local-mini-redis
```

### `benchmark-tool validate`

Validates manifests and result files against schemas.

```bash
benchmark-tool validate code/benchmarks/mini-redis/benchmark.toml
benchmark-tool validate benchmark-results/local-mini-redis
```

### Phase-One Implementation Note

The first implementation slice intentionally lands the local command-runner
surface before the full TCP benchmark surface:

- CLI parsing is driven by the repository's declarative Go `cli-builder`
  package, with the JSON CLI spec embedded into the `benchmark-tool` binary so
  the tool can be run locally from any directory.
- `validate` currently validates benchmark manifests. Result-directory schema
  validation remains future work.
- `run` initially executed only `driver = "command"` workloads. Phase three
  adds service startup and the first TCP/RESP workload driver.
- The initial implemented run flags were `--out`, `--trials`, and `--warmup`.
  Phase two adds subject checkout overrides and focused subject/workload
  filters. Seeds, build reuse, and CLI-level fail-fast overrides remain future
  work.
- Manifest `working_directory` values are resolved relative to the manifest's
  git repository root when available, so repo-local benchmark manifests work
  when the binary is invoked from outside the repository root.

### Phase-Two Implementation Note

The git-ref comparison slice now has a concrete preparation layer for command
benchmarks:

- `run` accepts repeated `--subject name=ref` flags to override a manifest
  subject's `checkout` value at the command line.
- `run` accepts `--subjects name,name` and `--workloads name,name` filters for
  focused local runs and smaller CI benchmark jobs.
- Any subject with a `checkout` value is prepared in a detached temporary git
  worktree, pinned to an exact commit SHA, and cleaned up when the run returns.
- Per-subject metadata is written to `subjects/<name>/subject.json`, and build
  logs remain in `subjects/<name>/build.log`.
- Randomized or paired trial ordering and richer in-result comparison reports
  remain future work.

### Phase-Three Implementation Note

The first TCP load-generation slice is implemented directly inside
`benchmark-tool` so the Mini Redis manifest can run locally and in CI without a
second binary yet:

- Service subjects allocate a loopback TCP port, replace `{port}` in the
  subject command, write `subjects/<name>/service.log`, and wait for
  `ready_check = "tcp-connect"`.
- `driver = "tcp-resp"` supports `one-shot`, `preconnect-then-fire`,
  `pipeline`, `idle`, and `hold` modes.
- RESP reads are frame-aware. The client parses complete RESP values, validates
  them against the manifest's expected frames, and marks the trial failed if any
  response is malformed or wrong.
- Per-connection samples include connect, write, first-byte, frame, total, and
  operation metrics; trial summaries include throughput and median client-side
  phase timings.
- `hold` workloads are intended for C10K-style capacity tests: they keep all
  connections open concurrently, wait for `hold_ms`, then validate that every
  surviving connection still responds.
- The standalone `benchmark-load-tcp` binary remains a future extraction point
  once the protocol and result contracts settle.

### Phase-Four Implementation Note

The first statistical-analysis slice upgrades `benchmark-tool compare` from a
summary-median printer into a verdict-producing comparison step:

- Comparisons read successful measurement trials from `trials.jsonl`, keeping
  the per-trial aggregate as the unit of comparison.
- The candidate result directory receives `comparison.json` and `comparison.md`
  so later automation and PR comments can consume the same verdict data.
- Each common subject/workload/metric row reports absolute difference, relative
  difference, a bootstrap confidence interval for the relative median
  difference, Cliff's delta, metric direction, and the practical threshold used.
- The default threshold is 5 percent for latency and throughput metrics, with
  startup metrics using 10 percent.
- Correctness failures suppress performance claims and produce a
  `correctness_failed` verdict before any speed/latency interpretation.
- The current implementation compares independent result directories. Paired
  trial ordering, outlier marking, and configurable thresholds remain future
  refinements.

---

## Git Ref Comparison

Comparing two code versions should be built in.

The tool should:

- create clean temporary worktrees for each git ref
- pin each subject to an exact commit SHA
- reject dirty benchmark subject worktrees unless explicitly allowed
- run each subject's build command in its own checkout
- keep build logs per subject
- run workloads in randomized or paired order
- tear down service processes after each trial

Example:

```bash
benchmark-tool run code/benchmarks/mini-redis/benchmark.toml \
  --subject current=HEAD \
  --subject baseline=origin/main
```

This is how we avoid arguing about whether one version had stale artifacts or
was accidentally built from a dirty tree.

---

## Cross-Language Comparison

Cross-language comparisons should work because each subject is just a command.

Example:

```toml
[[subjects]]
name = "rust-mini-redis"
kind = "service"
working_directory = "code/programs/rust/mini-redis"
build = "cargo build --release"
command = "./target/release/mini-redis --host 127.0.0.1 --port {port}"

[[subjects]]
name = "go-mini-redis"
kind = "service"
working_directory = "code/programs/go/mini-redis"
build = "go build -o mini-redis ."
command = "./mini-redis --host 127.0.0.1 --port {port}"

[[subjects]]
name = "python-mini-redis"
kind = "service"
working_directory = "code/programs/python/mini-redis"
build = "uv pip install -e .[dev]"
command = "uv run --no-project python -m mini_redis --host 127.0.0.1 --port {port}"
```

Rules:

- compare equivalent correctness behavior first
- use the same workload driver for all subjects
- record language runtime versions
- do not normalize away startup time unless the benchmark explicitly excludes it
- separate startup latency benchmarks from steady-state benchmarks
- state whether the benchmark includes interpreter startup, JIT warmup, or VM
  warmup

---

## Service Benchmark Rules

Service benchmarks need extra discipline.

### Readiness

A service subject must not receive load until it is ready.

Supported readiness checks:

- TCP connect succeeds
- HTTP health endpoint returns expected status
- stdout line matches a regex
- custom command exits successfully

### Correctness

Every workload must validate responses.

Examples:

- RESP `PING` must return `+PONG\r\n`
- Redis `SET` must return `+OK\r\n`
- Redis `GET` must return the expected bulk string
- IRC registration must receive expected welcome numerics
- WebSocket echo must return the same frame payload

Failed correctness samples are not performance wins. The report must place
correctness failures before speed summaries.

### Protocol Framing

The load generator must understand the response boundary.

For RESP:

- read exactly one RESP value unless the workload expects multiple values
- do not wait for EOF on persistent connections

For IRC:

- read complete CRLF-delimited lines

For HTTP/1:

- read by `Content-Length`, chunked encoding, or connection close according to
  the response headers

For WebSocket:

- read full frames, including continuation frames when supported

This rule exists because protocol-framing mistakes can add whole timeout
seconds to an otherwise fast server.

### Phases

Service workloads should be able to measure:

- startup time
- readiness time
- connect latency
- request write latency
- time to first byte
- time to full frame
- total request latency
- teardown time

The report should keep these separate. A slow connect phase and a slow handler
phase point to different fixes.

---

## First Benchmark Suite: Mini Redis Over `tcp-runtime`

The first real suite should live at:

```text
code/benchmarks/mini-redis/
```

Workloads:

| Workload | Purpose |
|---|---|
| `ping-one-shot-1000` | concurrent connection + one command + framed response |
| `ping-preconnect-1000` | isolates request/response from connect cost |
| `pipeline-set-get-100x100` | tests many commands over persistent sockets |
| `idle-10k` | tests mostly idle connection memory and polling behavior |
| `slow-client-1k` | tests backpressure and fairness |
| `fragmented-resp-1k` | tests partial frame buffering |
| `oversized-incomplete-frame` | tests security cap behavior |
| `large-response` | tests queued writes and flush fairness |

Primary metrics:

- `frame_ms` for request latency
- `requests_per_second` for throughput
- `connect_ms` for accept/connect pressure
- `server_cpu_percent` when available
- `max_resident_set_bytes` when available

Correctness checks:

- every response must parse as RESP
- every expected value must match
- oversized incomplete frames must produce the expected protocol error and
  close behavior
- no client may be counted as successful if it timed out waiting for a frame

---

## Reporting Format

The Markdown report should be concise enough for a PR but detailed enough for
engineering decisions.

Recommended sections:

```text
# Benchmark Report: mini-redis-tcp-runtime

## Verdict

## Environment

## Correctness

## Summary Table

## Latency Distributions

## Throughput

## Comparison

## Raw Artifacts
```

Example summary table:

| Workload | Subject | Correct | Median frame ms | p99 frame ms | Requests/s |
|---|---:|---:|---:|---:|---:|
| ping-one-shot-1000 | current | 100% | 6.1 | 17.3 | 9800 |
| ping-one-shot-1000 | baseline | 100% | 6.8 | 18.5 | 9100 |

Example verdict:

```text
current is faster than baseline for ping-one-shot-1000 frame latency.
Median improvement: 10.3 percent.
95 percent bootstrap CI: 6.1 percent to 14.2 percent.
Practical threshold: 5 percent.
Correctness: passed for both subjects.
```

If the confidence interval crosses zero:

```text
No clear winner. The observed median difference was 2.1 percent, but the
95 percent bootstrap CI ranged from -1.7 percent to 5.4 percent.
```

---

## CI and Automation Policy

Benchmarks should not run as ordinary required CI by default. Shared CI runners
are noisy, and performance gates can become flaky morale traps. The first
integration should be:

- local benchmark command for investigations
- optional PR comment report when explicitly requested
- scheduled benchmark automation on a stable runner in a future phase
- regression alerts only for large, sustained changes

Recommended modes:

| Mode | Trials | Use |
|---|---:|---|
| smoke | 5 | proves benchmark still works |
| local | 30 | development investigation |
| PR | 50 | requested PR comparison |
| nightly | 100+ | trend tracking |

### Separate GitHub Actions Workflow

Benchmarks should run in a separate workflow, not inside `.github/workflows/ci.yml`.

Recommended workflow:

```text
.github/workflows/benchmarks.yml
```

Reasons:

- correctness CI and performance measurement have different reliability needs
- benchmark runs may take longer than package tests
- benchmark failures should not block ordinary correctness fixes by default
- benchmark artifacts are different from build artifacts
- manual and scheduled triggers make more sense than "every push"
- future stable runners can be attached without disturbing normal CI

Initial triggers:

```yaml
on:
  workflow_dispatch:
    inputs:
      suite:
        description: Benchmark suite to run
        required: true
        default: mini-redis
      baseline:
        description: Baseline git ref
        required: true
        default: origin/main
      candidate:
        description: Candidate git ref
        required: true
        default: HEAD
      mode:
        description: smoke, local, pr, or nightly
        required: true
        default: smoke
  schedule:
    - cron: "0 8 * * *"
```

Optional later triggers:

- `issue_comment` for commands like `/benchmark mini-redis`
- `workflow_run` after CI passes on selected PRs
- `pull_request` only when benchmark manifests or runtime packages change, and
  only in smoke mode

If an `issue_comment` trigger is added later, it must be treated as an
untrusted request path:

- do not use `pull_request_target` to run benchmark code from a fork with a
  write-scoped token
- verify that the commenter has repository write permission before starting a
  benchmark
- check out the PR head with read-only permissions
- avoid exposing repository secrets to benchmarked code
- post results through a separate least-privilege step after the benchmark
  artifacts are produced

The safe default is manual `workflow_dispatch`; comment-triggered benchmarks
are an ergonomic layer that should only be added after the permission model is
explicitly implemented and reviewed.

The first workflow should:

- check out the repo with full history
- install only the benchmark toolchain needed by the selected suite
- build `benchmark-tool`
- run `benchmark-tool doctor`
- run `benchmark-tool run` with explicit baseline and candidate refs
- upload the whole result directory as an artifact
- publish `report.md` as a job summary
- never mark a PR as failed for a small or inconclusive performance delta

In a later phase, the workflow may comment on PRs, but the first implementation
should prefer uploaded artifacts and job summaries. That keeps the workflow
safe while the benchmark verdict rules mature.

### Runner Choice

GitHub-hosted runners are acceptable for smoke checks and rough comparisons,
but they are not ideal for serious statistical claims because CPU model,
machine load, thermal state, and noisy neighbors can vary between jobs.

Recommended tiers:

| Runner | Use |
|---|---|
| GitHub-hosted `ubuntu-latest` | smoke checks, harness validation |
| GitHub-hosted `macos-latest` | kqueue smoke coverage |
| GitHub-hosted `windows-latest` | Windows provider smoke coverage |
| self-hosted pinned machine | statistically meaningful trend tracking |
| self-hosted bare metal per OS | release-quality performance comparisons |

The workflow should record the runner type in `environment.json`, and reports
should label GitHub-hosted results as noisy unless a repeated schedule shows
stable trends.

---

## Failure Semantics

A benchmark can fail in several different ways. The tool must distinguish them.

| Failure | Meaning |
|---|---|
| `build_failed` | subject could not be built |
| `startup_failed` | service did not become ready |
| `correctness_failed` | workload ran but returned wrong result |
| `harness_failed` | load generator crashed or produced invalid samples |
| `timeout` | subject or request exceeded configured deadline |
| `inconclusive` | samples were too noisy or too few for a verdict |
| `regression` | statistically and practically worse |
| `improvement` | statistically and practically better |
| `no_clear_change` | no meaningful difference detected |

Performance summaries must not hide correctness failures.

---

## Implementation Phases

### Phase 0: Spec and Schemas

- add this spec
- add manifest and result schemas
- add example Mini Redis manifest

### Phase 1: Command Runner

- implement `benchmark-tool doctor`
- implement manifest validation
- run command-style microbenchmarks
- write `samples.jsonl`, `trials.jsonl`, and `environment.json`

### Phase 2: Git Ref Comparison

- create clean worktrees per subject ref
- run builds per subject
- randomize or pair trial order
- compare summary statistics
- generate Markdown reports

### Phase 3: TCP / RESP Load Generator

- implement TCP connection orchestration
- implement RESP frame-aware reads
- support one-shot, preconnect, pipeline, and idle workloads
- validate Mini Redis responses

### Phase 4: Statistical Analysis

- bootstrap confidence intervals
- effect sizes
- practical threshold verdicts
- outlier marking
- per-trial aggregate comparisons

### Phase 5: Reports and Trend Storage

- stable report Markdown
- optional HTML or SVG charts
- trend files for nightly benchmarks
- comparison against the last known good run

---

## Acceptance Criteria

The first useful benchmark tooling slice is complete when:

- a manifest can compare `HEAD` against `origin/main`
- the tool creates isolated worktrees for both refs
- both subjects can be built from clean checkouts
- Mini Redis can be started as a service subject
- the TCP load generator can run 1000 concurrent RESP `PING` clients
- the load generator reads RESP frames, not EOF
- raw samples and trial summaries are saved
- the report includes median, p99, confidence intervals, and verdicts
- correctness failures prevent performance wins
- the report states when there is no clear winner

---

## Open Questions

The first implementation can proceed without resolving all of these, but they
should stay visible.

- Should long-term benchmark trend storage live in git, artifacts, SQLite, or a
  separate dashboard?
- Should the first CLI be Go-only, or should a Rust load generator land first
  for TCP-specific realism?
- How much OS tuning should `doctor` recommend versus only report?
- Should benchmark manifests allow Docker subjects later?
- Should language-native benchmark adapters be generated by the scaffold
  generator?
- What machine should run scheduled benchmark automation if we want stable
  trends?

---

## The Guiding Principle

The benchmark tools should make performance conversations calmer.

Instead of asking:

```text
Why did this one script take 5.7 seconds?
```

we should be able to ask:

```text
Which phase got slower, by how much, with what confidence, under which
environment, compared to which commit, while still returning correct results?
```

That is the difference between vibes and engineering.
